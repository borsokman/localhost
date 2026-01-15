import { tryToActivateFinish } from "./finish.js";
import { enemies, flames, gridStep, halfStep, levelMap, timedEvents } from "./game.js";
import { Timer } from "./timer.js";

let timedCount = 0;
let enemyCount = 0;

export class Enemy {
    constructor(size, speed, x, y, name) {
        this.size = size;
        this.speed = speed;
        this.x = x + halfStep - size / 2; // Top left corner
        this.y = y + halfStep - size / 2;
        this.name = name;

        this.alive = true;
        this.onBomb = false;
        this.isMoving = false;

        this.element = document.createElement('div');
        this.element.id = `enemy${enemyCount}`;
        this.element.classList.add("enemy");
        this.element.style.width = `${size}px`;
        this.element.style.height = `${size}px`;
        this.element.style.borderRadius = `${size / 5}px`;
        this.element.style.transform = `translate(${this.x}px, ${this.y}px)`;
        document.getElementById("game-container").appendChild(this.element);

        // Instance-specific enemy walking and dying sound
        this.enemyWalking = new Audio("sfx/enemyWalking.mp3");
        this.enemyWalking.volume = 0.15;
        this.enemyWalking.loop = true;
        this.enemyWalking.play();
        this.enemyDeath = new Audio("sfx/enemyDeath.mp3");
        this.enemyDeath.volume = 0.3;

        let col = Math.round(x / gridStep);
        let row = Math.round(y / gridStep);

        // coordinates for enemy
        this.prevSpot = [this.x, this.y];
        this.curr = [row, col];
        this.next;
        this.direction = "spawn";
    }

    die() {
        this.element.classList.add('dead');
        this.enemyDeath.play();
        this.alive = false;
        this.enemyWalking.pause();
        this.enemyWalking.currentTime = 0;

        // Set a timer for removing the enemy from the game
        const countNow = timedCount;
        const timedDeath = new Timer(() => {
            this.element.remove();  // Remove the enemy element from the DOM
            timedEvents.delete(`enemyDeath${countNow}`);  // Clean up timed events
            enemies.delete(this.name);  // Remove from the enemies collection
            tryToActivateFinish();  // Check if the game should finish
        }, 1000);  // 1 second delay before removal

        timedEvents.set(`enemyDeath${countNow}`, timedDeath);  // Add the death event to the timer list
        timedCount++;  // Increment the timed count to track events
    };

    chooseDirection() {

        // update current first            
        this.curr = [Math.round((this.y + (this.size / 2) - halfStep) / gridStep), Math.round((this.x + (this.size / 2) - halfStep) / gridStep)]
        this.prevSpot = [this.curr[1] * gridStep + halfStep - this.size / 2, this.curr[0] * gridStep + halfStep - this.size / 2];

        // find directions with empty cells
        let availableDirs = []
        if (isEmpty(this.curr[0] - 1, this.curr[1])) availableDirs.push("up");
        if (isEmpty(this.curr[0] + 1, this.curr[1])) availableDirs.push("down");
        if (isEmpty(this.curr[0], this.curr[1] - 1)) availableDirs.push("left");
        if (isEmpty(this.curr[0], this.curr[1] + 1)) availableDirs.push("right");

        // don't go back the same way
        if (availableDirs.length > 1) {
            for (let i = 0; i < availableDirs.length; i++) {
                if (availableDirs[i] == "left" && this.direction == "right") {
                    availableDirs.splice(i, 1);
                    break;
                };
                if (availableDirs[i] == "right" && this.direction == "left") {
                    availableDirs.splice(i, 1);
                    break;
                };
                if (availableDirs[i] == "up" && this.direction == "down") {
                    availableDirs.splice(i, 1);
                    break;
                };
                if (availableDirs[i] == "down" && this.direction == "up") {
                    availableDirs.splice(i, 1);
                    break;
                };
            };
        };

        if (availableDirs.length > 0) {
            if (this.enemyWalking.paused) this.enemyWalking.play();

            this.direction = availableDirs[Math.floor(Math.random() * availableDirs.length)];

            if (this.direction == "left") this.next = [this.curr[0], this.curr[1] - 1];
            if (this.direction == "right") this.next = [this.curr[0], this.curr[1] + 1];
            if (this.direction == "up") this.next = [this.curr[0] - 1, this.curr[1]];
            if (this.direction == "down") this.next = [this.curr[0] + 1, this.curr[1]];
        } else {
            this.direction = "";
            if (!this.enemyWalking.paused) this.enemyWalking.play();
        }
    }

    moveEnemy(deltaTime) {
        if (this.alive) {
            let moveDistance = this.speed * deltaTime;

            // make enemy change direction if bomb is dropped in its way
            if (this.next && !isEmpty(this.next[0], this.next[1])) {
                if (!this.onBomb) {

                    [this.curr, this.next] = [this.next, this.curr];
                    let topLeftX = (this.curr[1] * gridStep) + halfStep - (this.size / 2);
                    let topLeftY = (this.curr[0] * gridStep) + halfStep - (this.size / 2);
                    this.prevSpot = [topLeftX, topLeftY];

                    if (this.direction == "left") this.direction = 'right';
                    else if (this.direction == "right") this.direction = 'left';
                    else if (this.direction == "up") this.direction = 'down';
                    else if (this.direction == "down") this.direction = 'up';

                    this.onBomb = true;
                }
            } else {
                this.onBomb = false;
            }

            // next position
            if (this.direction == "left") this.x -= moveDistance;
            if (this.direction == "right") this.x += moveDistance;
            if (this.direction == "up") this.y -= moveDistance;
            if (this.direction == "down") this.y += moveDistance;

            // Decide which way to go if at center of next square or stuck
            if (
                !this.direction ||
                this.direction == "spawn" ||
                Math.abs(this.x - this.prevSpot[0]) >= gridStep ||
                Math.abs(this.y - this.prevSpot[1]) >= gridStep
            ) {
                this.chooseDirection();
            };

            // Apply movement
            if (this.direction != "") {
                this.element.style.transform = `translate(${this.x}px, ${this.y}px)`;
            };

            // flames hit
            let enemyBounds = this.element.getBoundingClientRect()
            for (const flame of flames.values()) {
                if (checkHit(enemyBounds, flame)) {
                    this.die();
                    break;
                };
            };
        };

    };
};

function isEmpty(row, col) {
    return (
        row >= 0 && row <= 10 &&
        col >= 0 && col <= 12 &&
        !levelMap[row][col]
    );
}

function checkHit(enemyBounds, flame) {
    const flameBounds = flame.getBoundingClientRect();

    // No hit (false) if enemy is safely outside on at least one side
    return !(enemyBounds.right < flameBounds.left ||
        enemyBounds.left > flameBounds.right ||
        enemyBounds.bottom < flameBounds.top ||
        enemyBounds.top > flameBounds.bottom);
};