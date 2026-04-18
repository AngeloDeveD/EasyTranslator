import { getCurrentWindow } from '@tauri-apps/api/window';

import './titleBar.scss';

function Titlebar() {

    const appWindow = getCurrentWindow();

    return (
        <>
            <div data-tauri-drag-region className="titlebar">
                <div className="window-controls">
                    <button
                    className="window-controls window-controls-button controls-button-hide"
                    onClick={() => appWindow.minimize()}
                    ></button>
                    <button
                        className="window-controls window-controls-button controls-button-close"
                        onClick={() => appWindow.close()}
                    ></button>
                </div>
            </div>
        </>
  );
}

export default Titlebar;