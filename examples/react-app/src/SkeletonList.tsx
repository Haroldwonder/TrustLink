import React from "react";

function SkeletonItem(): React.ReactNode {
  return (
    <div className="att-item" style={{ opacity: 0.5 }}>
      <div className="row">
        <div style={{ width: "120px", height: "18px", background: "#2d3148", borderRadius: "4px", animation: "pulse 1.5s infinite" }} />
        <div style={{ width: "60px", height: "18px", background: "#2d3148", borderRadius: "4px", animation: "pulse 1.5s infinite" }} />
      </div>
      <div style={{ width: "200px", height: "14px", background: "#2d3148", borderRadius: "4px", marginTop: "8px", animation: "pulse 1.5s infinite" }} />
      <div style={{ width: "150px", height: "14px", background: "#2d3148", borderRadius: "4px", marginTop: "4px", animation: "pulse 1.5s infinite" }} />
    </div>
  );
}

export function SkeletonAttestationList({ count = 3 }: { count?: number }) {
  return (
    <div className="att-list">
      {Array.from({ length: count }).map((_, i) => (
        <SkeletonItem key={i} />
      ))}
    </div>
  );
}