import { bombs, bombTime, mult, gridStep, halfStep, levelMap, weakWalls, flames, timedEvents, powerUpMap, flamesPoolH, flamesPoolV } from "./game.js";
import { placeBomb, tickingBomb, wallBreak } from "./sounds.js";
import { Timer } from "./timer.js";

const gameContainer = document.getElementById("game-container");
let flameCounter = 0;
let timedCount = 0;

function isEdge(row, col) {
    return (row < 0 || row > 10 || col < 0 || col > 12);
};

function isWall(row, col) {
    return (
        row >= 0 && row <= 10 &&
        col >= 0 && col <= 12 &&
        levelMap[row][col] &&
        typeof levelMap[row][col] == 'string' &&
        (
            levelMap[row][col].startsWith('weakWall') ||
            levelMap[row][col] == 'solidWall'
        )
    );
};

function isBomb(row, col) {
    return (
        row >= 0 && row <= 10 &&
        col >= 0 && col <= 12 &&
        levelMap[row][col] &&
        Array.isArray(levelMap[row][col]) &&
        levelMap[row][col][0] == 'bomb'
    );
}

function isPowerUp(row, col) {
    return (
        row >= 0 && row <= 10 &&
        col >= 0 && col <= 12 &&
        powerUpMap[row][col] &&
        Array.isArray(powerUpMap[row][col]) &&
        (powerUpMap[row][col][0].startsWith('bombUp') || powerUpMap[row][col][0].startsWith('flameUp'))
    );
}

function horizontalFlame(bombsize, x, y) {
    const flame = flamesPoolH.find((f) => !f.active);

    flame.active = true;
    for (const ele of flame.elements) {
        ele.style.left = `${x + (bombsize / 2) - halfStep}px`;
        ele.style.top = `${y + (bombsize / 2) - (halfStep / 2)}px`;
        ele.style.display = "block";
    }
    flame.elements[1].style.clipPath = `inset(0)`;

    flameCounter++
    flames.set(`flameH${flameCounter}0`, flame.elements[0])   // to map of flames for collisions
    flames.set(`flameH${flameCounter}1`, flame.elements[1])

    const countNow = timedCount;
    const timedFlame = new Timer(() => {
        flame.active = false;
        flame.elements.forEach(e => e.style.display = "none");
        flames.delete(`flameH${flameCounter}0`);
        flames.delete(`flameH${flameCounter}1`);
        timedEvents.delete(`flameH${countNow}`)
    }, 500);
    timedEvents.set(`flameH${countNow}`, timedFlame)
    timedCount++;

    return flame.elements[1];
}

function verticalFlame(bombsize, x, y) {
    const flame = flamesPoolV.find((f) => !f.active);

    flame.active = true;
    for (const ele of flame.elements) {
        ele.style.left = `${x + (bombsize / 2) - (halfStep / 2)}px`;
        ele.style.top = `${y + (bombsize / 2) - halfStep}px`;
        ele.style.display = "block";
    }
    flame.elements[1].style.clipPath = `inset(0)`;

    flameCounter++
    flames.set(`flameV${flameCounter}0`, flame.elements[0])   // to map of flames for collisions
    flames.set(`flameV${flameCounter}1`, flame.elements[1])

    const countNow = timedCount;
    const timedFlame = new Timer(() => {
        flame.active = false;
        flame.elements.forEach(e => e.style.display = "none");
        flames.delete(`flameV${flameCounter}0`);
        flames.delete(`flameV${flameCounter}1`);
        timedEvents.delete(`flameV${countNow}`)
    }, 500);
    timedEvents.set(`flameV${countNow}`, timedFlame)
    timedCount++;

    return flame.elements[1];
}


export class Bomb {
    setValues(size, row, col, power, name) {
        // Align dropped bomb to grid
        this.mapCol = col;
        this.mapRow = row;
        this.x = this.mapCol * gridStep + halfStep - size / 2;
        this.y = this.mapRow * gridStep + halfStep - size / 2;
        this.size = size;
        this.owner = name;
        this.power = power;
    }

    constructor(size = mult * 60, row = 0, col = 0, power = 1, name = '') {
        this.setValues(size, row, col, power, name)
        this.active = false;

        this.element = document.createElement("div");
        this.element.classList.add("bomb");
        this.element.style.width = `${size}px`;
        this.element.style.height = `${size}px`;
        this.element.style.left = `${this.x}px`;
        this.element.style.top = `${this.y}px`;
        this.bounds = this.element.getBoundingClientRect();
        this.element.style.display = "none";

        this.explosion = new Audio("sfx/explosion.mp3");
        this.explosion.volume = 0.6;

        gameContainer.appendChild(this.element);
    };

    drop(row, col, power, name) {
        this.setValues(this.size, row, col, power, name)
        this.active = true;

        this.element.style.left = `${this.x}px`;
        this.element.style.top = `${this.y}px`;
        this.bounds = this.element.getBoundingClientRect();
        this.element.style.display = "block";

        bombs.set(`bomb${this.mapCol}${this.mapRow}`, this);  // add bomb to map for collision checks
        levelMap[this.mapRow][this.mapCol] = ['bomb', this];  // store reference to level map

        // Play sound when bomb is dropped
        placeBomb.play();

        // Start ticking sound
        tickingBomb.play();

        this.countNow = timedCount;
        const timedBomb = new Timer(() => {
            this.explode();
            timedEvents.delete(`bomb${this.countNow}`);
        }, bombTime);
        timedEvents.set(`bomb${this.countNow}`, timedBomb);
        timedCount++;
    }

    // explodeEarly removes the original timer and triggers the explosion
    explodeEarly() {
        if (timedEvents.has(`bomb${this.countNow}`)) {
            timedEvents.get(`bomb${this.countNow}`).cancel();
            timedEvents.delete(`bomb${this.countNow}`);
        }

        // small delay
        const timedEarlyExplotion = new Timer(() => {
            this.explode();
            timedEvents.delete(`earlyexplosion${this.countNow}`)
        }, 80);
        timedEvents.set(`earlyexplosion${this.countNow}`, timedEarlyExplotion);
        timedCount++;
    }

    explode() {
        this.element.classList.add('glowing');  // let css swap background
        this.explosion.play();

        // Stop ticking sound when bomb explodes
        tickingBomb.pause();
        tickingBomb.currentTime = 0; // Reset for next use

        // Draw flames of explosion in the middle
        horizontalFlame(this.size, this.x, this.y);
        verticalFlame(this.size, this.x, this.y);

        // Draw more flames in four directions
        const fourDirs = [
            { name: 'right', going: true, coords: undefined },
            { name: 'left', going: true, coords: undefined },
            { name: 'down', going: true, coords: undefined },
            { name: 'up', going: true, coords: undefined },
        ];
        let [lastLeft, lastRight, lastUp, lastDown] = [undefined, undefined, undefined, undefined];
        let firstWeakWall = true;

        for (let i = 1; i <= this.power; i++) {
            // In four directions: Stop flames at walls and edges, destroy weak walls, explode other bombs
            for (let j = 0; j < 4; j++) {
                if (fourDirs[j].name == 'right') fourDirs[j].coords = [this.mapRow, this.mapCol + i];
                if (fourDirs[j].name == 'left') fourDirs[j].coords = [this.mapRow, this.mapCol - i];
                if (fourDirs[j].name == 'down') fourDirs[j].coords = [this.mapRow + i, this.mapCol];
                if (fourDirs[j].name == 'up') fourDirs[j].coords = [this.mapRow - i, this.mapCol];

                if (fourDirs[j].going) {
                    let foundWall = false;
                    const dirRow = fourDirs[j].coords[0];
                    const dirCol = fourDirs[j].coords[1];

                    if (isWall(dirRow, dirCol)) {
                        if (levelMap[dirRow][dirCol].startsWith('weakWall')) {
                            this.destroyWall(dirRow, dirCol);
                            if (firstWeakWall) {
                                setTimeout(() => wallBreak.play(), 100);
                                firstWeakWall = false;
                            }
                        }
                        fourDirs[j].going = false;
                        foundWall = true;
                    };
                    if (isEdge(dirRow, dirCol)) {
                        fourDirs[j].going = false;
                    };
                    if (isBomb(dirRow, dirCol)) {
                        const bomb = levelMap[dirRow][dirCol][1];
                        levelMap[dirRow][dirCol] = '';
                        bomb.explodeEarly();
                    };
                    if (!foundWall && isPowerUp(dirRow, dirCol)) {
                        const powerUp = powerUpMap[dirRow][dirCol][1];
                        powerUp.burn();
                        fourDirs[j].going = false;
                    };
                };
            };

            // if still going, draw flames and save the most recent
            if (fourDirs[0].going) lastRight = horizontalFlame(this.size, this.x + gridStep * i, this.y);
            if (fourDirs[1].going) lastLeft = horizontalFlame(this.size, this.x - gridStep * i, this.y);
            if (fourDirs[2].going) lastDown = verticalFlame(this.size, this.x, this.y + gridStep * i);
            if (fourDirs[3].going) lastUp = verticalFlame(this.size, this.x, this.y - gridStep * i);

            // Cut off tip of full flame at the end to reveal rounded end
            if (fourDirs[0].going && lastRight && i == this.power) {
                lastRight.style.clipPath = `inset(0 ${20 * mult}px 0 0)`;
            }
            if (fourDirs[1].going && lastLeft && i == this.power) {
                lastLeft.style.clipPath = `inset(0 0 0 ${20 * mult}px)`;
            }
            if (fourDirs[2].going && lastDown && i == this.power) {
                lastDown.style.clipPath = `inset(0 0 ${20 * mult}px 0)`;
            }
            if (fourDirs[3].going && lastUp && i == this.power) {
                lastUp.style.clipPath = `inset(${20 * mult}px 0 0 0)`;
            }
        };

        // delay deleting bomb for a bit
        const timedExplotion = new Timer(() => {
            this.element.classList.remove('glowing');
            this.element.style.display = "none";
            this.active = false;

            bombs.delete(`bomb${this.mapCol}${this.mapRow}`);
            timedEvents.delete(`explosion${this.countNow}`);
            levelMap[this.mapRow][this.mapCol] = '';
        }, 500);
        timedEvents.set(`explosion${this.countNow}`, timedExplotion);
        timedCount++;
    };

    destroyWall(row, col) {
        let name = levelMap[row][col];
        weakWalls.get(name).collapse();

        const timedDeleteWall = new Timer(() => {
            weakWalls.delete(name);
            levelMap[row][col] = "";
            timedEvents.delete(`deleteWall${this.countNow}`)
        }, 500);

        timedEvents.set(`deleteWall${this.countNow}`, timedDeleteWall);
        timedCount++;
    };

    checkCollision(playerX, playerY, playerSize) {
        if (playerX + playerSize < this.x || playerX > this.x + this.size || playerY + playerSize < this.y || playerY > this.y + this.size) {
            // No collision: player is safely outside on at least one side, return input values
            return [playerX, playerY];
        } else {
            // find shortest direction out of collision
            const diffs = {
                x1: this.x - (playerX + playerSize),  // this left to player right
                x2: (this.x + this.size) - playerX,   // this right to player left
                y1: this.y - (playerY + playerSize),  // this top to player bottom
                y2: (this.y + this.size) - playerY    // this bottom to player top
            };

            // get key and value of item with lowest abs value
            let [lowestItems] = Object.entries(diffs).sort(([, v1], [, v2]) => Math.abs(v1) - Math.abs(v2));

            // modify inputs to place player just outside wall
            if (lowestItems[0].startsWith('x')) {
                return [playerX + lowestItems[1], playerY];
            } else {
                return [playerX, playerY + lowestItems[1]];
            };
        };
    };
};