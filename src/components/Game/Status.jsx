export default function Status({ status, text }) {
  const getColorClass = () => {
    switch (status) {
      case "success": return "status success";
      case "error":   return "status error";
      case "update":  return "status update";
      default:        return "status success";
    }
  };

  return (
    <div className={getColorClass()}>
      <span className="dot" />
      {text}
    </div>
  );
}