# 中文转录简繁体稳定性修复

## 状态：已完成 ✅

## 问题描述

Whisper 语音识别在中文转录时，输出结果在简体中文和繁体中文之间不稳定切换。

## 根因分析

Whisper 模型的训练数据同时包含简体和繁体中文。语言代码 `"zh"` 不区分简繁体，模型推理时根据上下文概率选择字符，导致输出不稳定。

参考：[OpenAI Whisper Discussion #277](https://github.com/openai/whisper/discussions/277)

## 技术方案

通过 `FullParams::set_initial_prompt()` 设置含简体字符的引导 prompt，利用 prompt 中的字符风格（"话" vs "話"）引导模型倾向输出对应字体。

当 `language = "zh"` 时，自动设置 `initial_prompt = "以下是普通话的句子。"`。

## 改动文件

| 文件 | 改动 |
|------|------|
| `src-tauri/src/whisper/transcriber.rs` | 新增 `initial_prompt_for_language()` 函数，在 `transcribe()` 中调用设置 initial_prompt |
| `src-tauri/src/whisper/tests.rs` | 新增 2 个测试用例验证 initial_prompt 映射逻辑 |

## 扩展说明

- 更大的模型（medium / large）对 initial_prompt 引导的响应更稳定
- 已扩展：繁体中文输出实现见 [feature-traditional-chinese.md](feature-traditional-chinese.md)。当 `language = "zh-TW"` 时返回 `"以下是國語的句子。"` 引导繁体输出
- 若 initial_prompt 仍不够稳定，可考虑后处理方案（opencc 简繁转换库）
