import { gridStep, halfStep } from "./game.js";

const gameContainer = document.getElementById("game-container");

export class FlameH {
    constructor(bombsize, x, y) {

        this.active = false;

        this.elements = [];
        for (let i = 0; i < 2; i++) {
            let flame = document.createElement('div');
            flame.classList.add("flame");
            flame.classList.add("horizontal");
            if (i == 0) flame.classList.add("ends");
            flame.style.width = `${gridStep}px`;
            flame.style.height = `${halfStep}px`;
            flame.style.left = `${x + (bombsize / 2) - halfStep}px`;
            flame.style.top = `${y + (bombsize / 2) - (halfStep / 2)}px`;
            flame.style.display = "none";

            this.elements.push(flame);
            gameContainer.appendChild(flame);
        }
    };
};

export class FlameV {
    constructor(bombsize, x, y) {

        this.active = false;

        this.elements = [];
        for (let i = 0; i < 2; i++) {
            let flame = document.createElement('div');
            flame.classList.add("flame");
            flame.classList.add("vertical");
            if (i == 0) flame.classList.add("ends");
            flame.style.width = `${halfStep}px`;
            flame.style.height = `${gridStep}px`;
            flame.style.left = `${x + (bombsize / 2) - (halfStep/2)}px`;
            flame.style.top = `${y + (bombsize / 2) - halfStep}px`;
            flame.style.display = "none";

            this.elements.push(flame);
            gameContainer.appendChild(flame);
        }
    };
};
