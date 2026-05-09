/* global React */

// Reusable atoms shared by machine.jsx, autocraft.jsx, lookup.jsx, integrated.jsx
const Slot = ({ filled, active, label, qty, icon, style }) => (
  <div className={`sk-slot ${filled ? "sk-filled" : ""} ${active ? "sk-active" : ""}`} style={style}>
    {icon && <span className="sk-icon">{icon}</span>}
    {qty != null && <span className="sk-qty">{qty}</span>}
    {label && <span className="sk-label">{label}</span>}
  </div>
);

const Row = ({ children, gap = 4, style }) => (
  <div style={{ display: "flex", gap, alignItems: "center", ...style }}>{children}</div>
);

const Col = ({ children, gap = 4, style }) => (
  <div style={{ display: "flex", flexDirection: "column", gap, ...style }}>{children}</div>
);
