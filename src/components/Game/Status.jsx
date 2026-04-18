export default function Status({ status }) {
  const config = {
    success: { text: "All up to date", className: "status success" },
    update: { text: "Update available", className: "status update" },
    error: { text: "Conflicts detected", className: "status error" },
  };

  const { text, className } = config[status];

  return (
    <div className={className}>
      <span className="dot" />
      {text}
    </div>
  );
}