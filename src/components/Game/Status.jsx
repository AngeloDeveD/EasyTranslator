export default function Status({ status, text }) {
  // Маппинг доменного статуса в CSS-класс для единообразной цветовой индикации.
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
