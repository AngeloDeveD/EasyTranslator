[![Русский](https://img.shields.io/badge/Язык-Русский-green.svg)](README_rus.md)
[![Roadmap](https://img.shields.io/badge/Roadmap-00A2ff)](ROADMAP.md)

# EasyTranslator: Simple Translation Installation for Games

**EasyTranslator** is a program that lets you install a localization with a language not officially supported in the game itself. You don't need to search for a translation for your game every time. The only thing you need to do is launch the program, select the game, and click the **Install** button — and you're done! The program will handle everything else for you.

## ⚙️ How It Works

1. Launch the program.
2. Type the game's name into the search bar.
3. Choose the appropriate icon from the list.
4. Click "Install". The program will automatically download and extract the translation to the correct location.

## 🏗 Simple Interface
Initially, you'll be presented with a selection of games whose translations have been recently updated. To find your game, simply type the name of your project into the search bar and select your game's icon.

## 🛠 Development Stack
Languages: Rust 1.94.1, JavaScript / TypeScript  
Frameworks: Tauri v2, React

## 🚀 Installation and Running (For Developers)
1. Install Node.js and Rust.
2. Clone the repository:
```bash
git clone https://github.com/AngeloDeveD/EasyTranslator.git
cd EasyTranslator
```
3. Install dependencies and run:
```bash
npm install
npm run tauri dev
```

4. To create a build, run:
```bash
npm run tauri build
```

## 📁 Project Structure
**Let's leave this for a bit later 😊**