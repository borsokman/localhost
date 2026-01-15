import { Bomb } from "./bomb.js";
import { Enemy } from "./enemy.js";
import { FlameH, FlameV } from "./flames.js";
import { enemies, gridStep, halfStep, level, levelMap, mult, powerUpMap, powerups, solidWalls, weakWalls, flamesPoolV, flamesPoolH, bombsPool } from "./game.js";
import { Player } from "./player.js";
import { BombUp, FlameUp } from "./powerup.js";
import { SolidWall, WeakWall } from "./walls.js";

export function resizeGameContainer(level) {
    const gameContainer = document.getElementById("game-container");

    const windowWidth = window.innerWidth;
    const windowHeight = window.innerHeight;

    // wide or narrow window? single screen Bomberman level is 13 * 11 squares
    if (windowWidth / windowHeight > 13 / 11) {
        gameContainer.style.height = windowHeight * 0.8 + "px";
        gameContainer.style.width = windowHeight * 0.8 * (13 / 11) + "px";
    } else {
        gameContainer.style.height = windowWidth * 0.8 * (11 / 13) + "px";
        gameContainer.style.width = windowWidth * 0.8 + "px";
    };

    const bounds = gameContainer.getBoundingClientRect();
    gameContainer.style.left = (windowWidth - bounds.width) / 2 + 'px';
    gameContainer.style.top = (windowHeight - bounds.height) / 2 + 'px';


    // Remove the previous level class if it exists
    gameContainer.classList.remove(`level-${level - 1}`);

    // Apply the level class to the game container
    gameContainer.classList.add(`level-${level}`);

    return bounds;
};

export function getGridSize() {
    const gameContainer = document.getElementById("game-container");
    const gridStep = gameContainer.getBoundingClientRect().width / 13;
    const halfStep = gridStep / 2;
    return [gridStep, halfStep];
};

export function setUpGame(bounds) {
    // multiplier from game-container size scales things (speed, placements) 
    // to different sized windows
    const multiplier = bounds.width / 1000;

    const playerSpeed = 4.5 * multiplier;
    const playerSize = 55 * multiplier;
    const playerX = halfStep - (playerSize / 2); // put player to top left    
    const playerY = halfStep - (playerSize / 2);

    const player = new Player(playerSize, playerSpeed, playerX, playerY);

    return [multiplier, player];
};

export function makeLevelMap() {
    // 11 rows and 13 columns
    let map = new Array(11);
    for (let i = 0; i < map.length; i++)  map[i] = new Array(13);
    return map;
};

export function makeWalls(level) {

    // place 6 * 5 solid walls inside play area
    for (let i = 0; i < 6; i++) {
        for (let j = 0; j < 5; j++) {
            const mapX = (1 + i * 2);
            const mapY = (1 + j * 2);
            const x = gridStep * mapX;
            const y = gridStep * mapY;
            // Create SolidWall instance with level passed
            const newSolid = new SolidWall(x, y, gridStep, level);
            solidWalls.push(newSolid);

            levelMap[mapY][mapX] = 'solidWall';
        };
    };

    // put solid walls around play area
    const yVals = [-1, 11];
    for (let i = 0; i < 15; i++) {
        for (const yVal of yVals) {
            const mapX = i - 1;
            const mapY = yVal;
            const x = gridStep * mapX;
            const y = gridStep * mapY;
            new SolidWall(x, y, gridStep, level);
        }
    };
    const xVals = [-1, 13];
    for (let i = 0; i < 11; i++) {
        for (const xVal of xVals) {
            const mapX = xVal
            const mapY = i;
            const x = gridStep * mapX;
            const y = gridStep * mapY;
            new SolidWall(x, y, gridStep, level);
        }
    };

    // place weak walls randomly
    while (weakWalls.size < 45) {
        const mapX = Math.floor(Math.random() * 13);
        const mapY = Math.floor(Math.random() * 11);

        // don't replace content or put anything in the top left and bottom right corners
        if (levelMap[mapY][mapX] || (mapX < 2 && mapY < 2) || (mapX > 10 && mapY > 8)) {
            continue;
        };

        const x = gridStep * mapX;
        const y = gridStep * mapY;
        const name = `weakWall${String(mapX).padStart(2, '0')}${String(mapY).padStart(2, '0')}`;
        const newWeak = new WeakWall(x, y, gridStep, level);

        weakWalls.set(name, newWeak);
        levelMap[mapY][mapX] = name;
    };

    // place bomb powerups inside weak walls
    while (powerups.size < 5) {
        const mapX = Math.floor(Math.random() * 13);
        const mapY = Math.floor(Math.random() * 11);

        if (levelMap[mapY][mapX] &&
            typeof levelMap[mapY][mapX] == 'string' &&
            levelMap[mapY][mapX].startsWith('weakWall') &&
            !powerUpMap[mapY][mapX]
        ) {
            const x = gridStep * mapX;
            const y = gridStep * mapY;
            const name = `bombUp${String(mapX).padStart(2, '0')}${String(mapY).padStart(2, '0')}`;
            const newBombUp = new BombUp(x, y, gridStep * 1.0, name, mapY, mapX);
            powerups.set(name, newBombUp)
            powerUpMap[mapY][mapX] = [name, newBombUp];
        };
    }

    // place flame powerups inside weak walls
    while (powerups.size < 10) {
        const mapX = Math.floor(Math.random() * 13);
        const mapY = Math.floor(Math.random() * 11);

        if (levelMap[mapY][mapX] &&
            typeof levelMap[mapY][mapX] == 'string' &&
            levelMap[mapY][mapX].startsWith('weakWall') &&
            !powerUpMap[mapY][mapX]
        ) {
            const x = gridStep * mapX;
            const y = gridStep * mapY;
            const name = `flameUp${String(mapX).padStart(2, '0')}${String(mapY).padStart(2, '0')}`;
            const newFlameUp = new FlameUp(x, y, gridStep * 1.0, name, mapY, mapX);
            powerups.set(name, newFlameUp)
            powerUpMap[mapY][mapX] = [name, newFlameUp];
        };
    }

    // place enemies
    while (enemies.size < 1 + (level * 1.5)) {
        const mapX = Math.floor(Math.random() * 13);
        const mapY = Math.floor(Math.random() * 11);

        // don't replace content or put anything in the top left
        if (levelMap[mapY][mapX] || (mapX < 3 && mapY < 3)) {
            continue;
        };

        const x = gridStep * mapX;
        const y = gridStep * mapY;
        const name = `enemy${String(mapX).padStart(2, '0')}${String(mapY).padStart(2, '0')}`;
        const newEnemy = new Enemy(55 * mult, level * mult, x, y, name);
        enemies.set(name, newEnemy);
        levelMap[mapY][mapX] = 'enemy';
    };

    // enemies were there only to stop them being placed on top of each other
    for (let i = 0; i < levelMap.length; i++) {
        for (let j = 0; j < levelMap[0].length; j++) {
            if (levelMap[i][j] == 'enemy') {
                levelMap[i][j] = null;
            };
        };
    };
};

export function makeTextBar() {
    const gameArea = document.getElementById("game-container").getBoundingClientRect();
    let oldTextBar = document.querySelector(".textbar");

    const pad = 10;
    if (!oldTextBar) {
        // one bar to contain all text
        let textbar = document.createElement('div');
        textbar.classList.add("textbar");
        textbar.style.height = `${gridStep - pad * 2 * mult}px`;
        textbar.style.width = `${gridStep * 13 - pad * 2 * mult}px`;
        textbar.style.left = `${gameArea.left}px`;
        textbar.style.top = `${gameArea.top - gridStep}px`;
        textbar.style.padding = `${pad * mult}px`;

        // four smaller bits to display info
        const infos = [];
        const ids = ["levelinfo", "livesinfo", "scoreinfo", "timeinfo"];
        const placeholders = ["Level: 1", "Lives: X", "Score: 0", "00:00"]
        for (let i = 0; i < 4; i++) {
            let info = document.createElement('div');
            info.classList.add("infobox");
            info.style.margin = `${pad * mult}px`;
            info.style.padding = `${pad * mult}px`;
            info.style.borderWidth = `${mult * 2}px`;
            info.style.borderRadius = `${pad * mult}px`;
            info.id = ids[i];
            info.textContent = placeholders[i];
            info.style.fontSize = `${18 * mult}px`;
            textbar.appendChild(info);
            infos.push(info);
        }

        infos[3].style.justifyContent = "center";
        document.body.appendChild(textbar);

        return infos;
    } else {
        // recalculate text bar size and position in case window was resized
        oldTextBar.style.height = `${gridStep - pad * 2 * mult}px`;
        oldTextBar.style.width = `${gridStep * 13 - pad * 2 * mult}px`;
        oldTextBar.style.left = `${gameArea.left}px`;
        oldTextBar.style.top = `${gameArea.top - gridStep}px`;
        oldTextBar.style.padding = `${pad * mult}px`;

        return [
            document.getElementById("levelinfo"),
            document.getElementById("livesinfo"),
            document.getElementById("scoreinfo"),
            document.getElementById("timeinfo")
        ];
    };
}

export function fillFlameAndBombPools() {
    const bombSize = mult * 60;

    for (let i = 0; i < 200; i++) {
        flamesPoolH.push(new FlameH(bombSize, 0, 0));
        flamesPoolV.push(new FlameV(bombSize, 0, 0));
    }

    for (let i = 0; i < 50; i++) {
        bombsPool.push(new Bomb(bombSize));
    }
}
