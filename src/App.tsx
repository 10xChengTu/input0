import { BrowserRouter, Routes, Route } from "react-router-dom";
import Settings from "./pages/Settings";
import Overlay from "./pages/Overlay";

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Settings />} />
        <Route path="/overlay" element={<Overlay />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
