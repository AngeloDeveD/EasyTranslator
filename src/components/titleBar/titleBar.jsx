import { getCurrentWindow } from '@tauri-apps/api/window';

import './titleBar.scss';

function Titlebar() {

    // Управление окном идет через Tauri API, а не через browser window controls.
    const appWindow = getCurrentWindow();

    return (
        <>
            <div data-tauri-drag-region className="titlebar">
                <div className="window-controls">
                    <button
                    className="window-controls window-controls-button controls-button-hide"
                    // Только сворачивание: maximize отключен в конфиге окна.
                    onClick={() => appWindow.minimize()}
                    ></button>
                    <button
                        className="window-controls window-controls-button controls-button-close"
                        // Завершает приложение/окно.
                        onClick={() => appWindow.close()}
                    ></button>
                </div>
            </div>
        </>
  );
}

export default Titlebar;
