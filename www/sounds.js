// Sound effects
export const menuMusic = new Audio("sfx/menuMusic.mp3");
menuMusic.loop = true;
export const walkingSound = new Audio("sfx/playerWalking.mp3");
walkingSound.volume = 0.6;
walkingSound.loop = true;
export const playerDeath = new Audio("sfx/playerDeath.mp3");
playerDeath.volume = 0.3;
export const playerDeath2 = new Audio("sfx/playerDeath2.mp3");
playerDeath2.volume = 0.3;
window.deathSound = 0;
export const playerBombDeath = new Audio("sfx/playerBombDeath.mp3");
playerBombDeath.volume = 0.5;
export const placeBomb = new Audio("sfx/placeBomb.mp3");
export const tickingBomb = new Audio("sfx/tickingBomb.mp3");
tickingBomb.loop = true;
export const wallBreak = new Audio("sfx/wallBreak.mp3");
wallBreak.volume = 0.6;
export const finishLevel = new Audio("sfx/finishLevel.mp3");
export const gameLost1 = new Audio("sfx/sad-trombone.mp3");
export const gameLost2 = new Audio("sfx/sinister-laugh.mp3");
export const congrats = new Audio("sfx/congratulations.mp3");
export const crowdClapCheer = new Audio("sfx/cheering-and-clapping-crowd.mp3");

// Background music for each level
export const levelMusic = [
    new Audio('sfx/level1music.mp3'),
    new Audio('sfx/level2music.mp3'),
    new Audio('sfx/level3music.mp3'),
    new Audio('sfx/level4music.mp3'),
    new Audio('sfx/level5music.mp3')
];

levelMusic.forEach((aud) => aud.loop = true);
