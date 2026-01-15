import { halfStep, powerUpMap, powerups, timedEvents } from "./game.js";
import { Timer } from "./timer.js";

let timedCount = 0;

class PowerUp {
    constructor(x, y, size, nameOf, row, col) {
        this.x = x + halfStep - (size / 2);
        this.y = y + halfStep - (size / 2);
        this.size = size;
        this.row = row;
        this.col = col;

        this.name = nameOf;

        this.element = document.createElement("div");
        this.element.classList.add("powerup")
        this.element.style.position = "absolute";
        this.element.style.width = `${size}px`;
        this.element.style.height = `${size}px`;
        this.element.style.left = `${this.x}px`;
        this.element.style.top = `${this.y}px`;

        document.getElementById("game-container").appendChild(this.element);
    };

    checkCollision(playerX, playerY, playerSize) {
        return (!
            (
                playerX + playerSize < this.x ||
                playerX > this.x + this.size ||
                playerY + playerSize < this.y ||
                playerY > this.y + this.size
            )
        )
    };

    pickUp() {
        this.sound.play();
        this.element.remove();
        powerUpMap[this.row][this.col] = '';
        powerups.delete(this.name);
    }

    burn() {
        this.element.style.backgroundImage = `url("images/burn.svg")`;
        const countNow = timedCount;
        const timedCollapse = new Timer(() => {
            this.element.remove(); // Silent removal, no sound
            powerUpMap[this.row][this.col] = '';
            powerups.delete(this.name);
            //this.pickUp();
            timedEvents.delete(`burnPowerUp${countNow}`)
        }, 500);
        timedEvents.set(`burnPowerUp${countNow}`, timedCollapse)
        timedCount++;
    }
};

export class BombUp extends PowerUp {
    constructor(x, y, size, nameOf, row, col) {
        super(x, y, size, nameOf, row, col);
        this.powerType = "bomb";
        this.element.classList.add("bombup");
        this.sound = new Audio("sfx/bombUp.mp3");
    };
    pickUp() {
        super.pickUp();
    };
};

export class FlameUp extends PowerUp {
    constructor(x, y, size, nameOf, row, col) {
        super(x, y, size, nameOf, row, col);
        this.powerType = "flame";
        this.element.classList.add("flameup");
        this.sound = new Audio("sfx/flameUp.mp3");
    };
    pickUp() {
        super.pickUp();
    };
};
