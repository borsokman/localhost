import { enemies, finish, mult } from "./game.js";

export class Finish {
    constructor(x, y, size) {
        this.x = x;
        this.y = y;
        this.size = size;
        this.element = document.createElement("div");
        this.element.classList.add("finish")

        this.active = false;

        this.element.style.width = `${size}px`;
        this.element.style.height = `${size}px`;
        this.element.style.left = `${x}px`;
        this.element.style.top = `${y}px`;

        document.getElementById("game-container").appendChild(this.element);
    };

    checkCollision(playerX, playerY, playerSize) {
        // is player is fully inside finish square, with some generosity added
        return (
            playerX + (5 * mult) >= this.x &&
            playerX + playerSize - (5 * mult) <= this.x + this.size &&
            playerY + (5 * mult) >= this.y &&
            playerY + playerSize - (5 * mult) <= this.y + this.size
        )
    };

    makeActive() {
        this.active = true;
        this.element.style.backgroundImage = `url("images/finish.svg")`;
    }
};

export function tryToActivateFinish() {
    if (enemies.size == 0) {
        finish.makeActive();
    }
};