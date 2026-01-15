import { Finish } from "./finish.js";
import { resizeGameContainer, getGridSize, setUpGame, makeWalls, makeLevelMap, makeTextBar, fillFlameAndBombPools } from "./initialize.js";
import { congrats, crowdClapCheer, levelMusic, menuMusic, tickingBomb, walkingSound } from "./sounds.js";

export let bounds;
export let mult = 1.0;
export let gridStep = 0;
export let halfStep = 0;
export let levelMap;                    // for placing elements, wall collapses
export let powerUpMap;

export let solidWalls = [];             // for player collisions
export const weakWalls = new Map();     // for player collisions
export const bombs = new Map();         // for player collisions
export const bombTime = 2500;
export const flames = new Map();        // for player collisions
export const timedEvents = new Map();
export const enemies = new Map();       // for player collisions
export const powerups = new Map();      // for player collisions
export let flamesPoolH = [];              // pools of objects to avoid run time memory allocations
export let flamesPoolV = [];
export let bombsPool = [];
export let finish;

let levelinfo;
let livesinfo;
let scoreinfo;
let timeinfo;
const twoMinutes = 120000;
let score = 0;
let timeToSubtract = 0;
let gameStartTime;

let player;
let paused = false;
let finished = false;
let gameRunning = false;
let gameLost = false;
let scoreTime = 0;
let lastFrameTime = 0;
export let level = 1;
let currentMusic;

// Prevent default behavior for arrow keys to avoid page scrolling. Notice 'window'
window.addEventListener("keydown", function (e) {
    if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(e.key)) {
        e.preventDefault();
    };
});

export function restartGame() {
    location.reload();
};

export function toggleFinished() {
    finished = !finished;
    scoreTime = window.performance.now() - timeToSubtract;
}

function toggleEndScreen() {
    const victoryScreen = document.getElementById("victory");
    let msg = document.getElementById("victory-message");
    msg.textContent = `You scored ${Math.round(score)} points with ${player.lives} lives remaining, you absolute legend!`;
    victoryScreen.style.display == "flex" ? victoryScreen.style.display = "none" : victoryScreen.style.display = "flex";
}

export function nextLevel() {

    let scoreAddition = ((twoMinutes - scoreTime) / 1000) * player.lives * level;
    if (scoreAddition > 0) score += scoreAddition;
    updateScoreInfo(score);

    if (level >= 5) {
        toggleEndScreen();
        congrats.play();
        congrats.onended = () => {
            crowdClapCheer.play();
        };
        return;
    }

    document.getElementById("game-container").replaceChildren();

    level++;
    timeToSubtract = window.performance.now(); // resets level clock  
    solidWalls = [];
    weakWalls.clear();
    bombs.clear();
    flames.clear();
    timedEvents.clear();
    enemies.clear();
    powerups.clear();
    flamesPoolH = [];
    flamesPoolV = [];
    bombsPool = [];

    //loadLevel();
    startSequence();
    updateLevelInfo(level);
    updateLivesInfo(player.lives);
    toggleFinished();
};

function togglePause() {
    if (gameRunning && !gameLost) {
        paused = !paused;
        const pauseMenu = document.getElementById("pause-menu");

        if (paused) {
            pauseMenu.style.display = "block";
            for (const timed of timedEvents.values()) {
                timed.pause();
            }
            if (currentMusic) {
                currentMusic.pause();
            }
            walkingSound.pause();
            enemies.forEach(enemy => {
                enemy.enemyWalking.pause();
            });
            tickingBomb.pause();
            timeToSubtract -= window.performance.now(); // stored for unpausing 
        } else {
            pauseMenu.style.display = "none";
            for (const timed of timedEvents.values()) {
                timed.resume();
            }
            if (currentMusic) {
                currentMusic.play();
            }
            if (player.isMoving) {
                walkingSound.play();
            }
            enemies.forEach(enemy => {
                if (enemy.isMoving) {
                    enemy.enemyWalking.play();
                }
            });
            if (bombs.size > 0) {
                tickingBomb.play();
            }
            timeToSubtract += window.performance.now(); // this is used to display time
            updateStartTime();
        };
    };
};

document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
        togglePause();
    }
});

function updateTimeInfo(time) {
    time = twoMinutes - time;
    if (time < 0) time = 0;
    let totalSeconds = Math.floor(time / 1000);
    let minutes = Math.floor(totalSeconds / 60); // Get minutes
    let seconds = totalSeconds % 60; // Get seconds
    timeinfo.textContent = `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

function updateLevelInfo(level) {
    levelinfo.textContent = `Level: ${level}`
}

export function updateLivesInfo(lives) {
    let livesText = '';
    for (let i = 0; i < lives; i++) {
        livesText += `❤️`;
    };
    livesinfo.textContent = 'Lives: ' + livesText;
}

function updateScoreInfo(score) {
    if (score < 0) score = 0;
    scoreinfo.textContent = `Score: ${Math.round(score)}`
}

function updateStartTime() {
    gameStartTime = window.performance.now() + 100;     // time buffer to load something
}

function startSequence() {
    const gameContainer = document.getElementById("game-container");
    gameContainer.style.visibility = "hidden";

    const startMenu = document.getElementById("start-menu");
    let tasks = [
        () => { bounds = resizeGameContainer(level); },
        () => { [gridStep, halfStep] = getGridSize();[mult, player] = setUpGame(bounds) },
        () => { levelMap = makeLevelMap(); powerUpMap = makeLevelMap(); },
        () => { makeWalls(level); },
        () => { fillFlameAndBombPools(); },
        () => { finish = new Finish(gridStep * 12, gridStep * 10, gridStep); },
        () => {
            if (currentMusic) { currentMusic.pause(); currentMusic.currentTime = 0; }
            currentMusic = levelMusic[level - 1]; currentMusic.play();
        },
        () => {
            menuMusic.pause();
            menuMusic.currentTime = 0;
            startMenu.style.display = "none";
        },
        () => {
            updateStartTime();
            [levelinfo, livesinfo, scoreinfo, timeinfo] = makeTextBar();
            updateLivesInfo(player.lives);
        },
        () => { document.body.classList.add("grey"); },
        () => {
            const gameContainer = document.getElementById("game-container");
            gameContainer.style.visibility = "visible";
        },
        () => { runGame(); }
    ];

    function processNextTask() {
        if (tasks.length > 0) {
            let task = tasks.shift();
            task();
            requestAnimationFrame(processNextTask);
        }
    }

    requestAnimationFrame(processNextTask);


}


export function setGameLost() {
    gameLost = true;
}

function runGame() {
    const now = window.performance.now();
    timeToSubtract = now;
    lastFrameTime = now; // initialize to current timestamp
    gameRunning = true;
    requestAnimationFrame(gameLoop);

    function gameLoop(timestamp) {
        let deltaTime = (timestamp - lastFrameTime) / 16.7; // use deltaTime to normalize speed for different refresh rates
        lastFrameTime = timestamp;

        // pause game when window not active
        window.onblur = () => { if (!paused) togglePause() };

        if (timestamp > gameStartTime && !paused && !gameLost) {
            if (!finished) updateTimeInfo(timestamp - timeToSubtract);
            player.movePlayer(deltaTime);
            enemies.forEach((en) => en.moveEnemy(deltaTime));
        }

        // requestAnimationFrame() always runs callback with 'timestamp' argument (milliseconds since the page loaded)
        if (gameRunning) { // Keep looping unless explicitly stopped
            requestAnimationFrame(gameLoop);
        };
    };
};

function playMenuMusicOnInteraction() {
    menuMusic.play();
    // Remove the event listeners after the first interaction to avoid triggering play multiple times
    document.removeEventListener('click', playMenuMusicOnInteraction);
    document.removeEventListener('keydown', playMenuMusicOnInteraction);
}

document.addEventListener("DOMContentLoaded", () => {
    // Pause menu
    document.getElementById("continue-btn").addEventListener("click", togglePause);
    const restarts = document.querySelectorAll('.restart-btn');
    restarts.forEach(rs => rs.addEventListener('click', restartGame));

    document.getElementById("restart-btn-game-over").addEventListener("click", () => {
        document.getElementById("game-over-menu").style.display = "none";
        restartGame();
    });

    // Start music on interaction to avoid errors
    document.addEventListener('click', playMenuMusicOnInteraction);
    document.addEventListener('keydown', playMenuMusicOnInteraction);

    // Start menu
    const startMenu = document.getElementById("start-menu");
    startMenu.style.display = "block";
    //menuMusic.play();    

    document.getElementById("start-btn").addEventListener("click", () => {
        startSequence();
    });
});
