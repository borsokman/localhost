import { bombTime, bombs, bounds, enemies, finish, flames, nextLevel, powerups, solidWalls, timedEvents, weakWalls, levelMap, updateLivesInfo, gridStep, toggleFinished, setGameLost, bombsPool, mult } from "./game.js";
import { finishLevel, gameLost1, gameLost2, levelMusic, playerBombDeath, playerDeath, playerDeath2, walkingSound } from "./sounds.js";
import { Timer } from "./timer.js";

let timedCount = 0;

export class Player {
    constructor(size, speed, x, y) {
        this.size = size;
        this.speed = speed;
        this.startX = x;
        this.startY = y;
        this.x = x;
        this.y = y;

        this.lives = 5;
        this.alive = true;
        this.bombAmount = 1;
        this.bombPower = 2;

        this.element = document.createElement('div');
        this.element.id = "player";
        this.element.style.width = `${size}px`;
        this.element.style.height = `${size}px`;
        this.element.style.position = 'absolute';
        this.element.style.transform = `translate(${x}px, ${y}px)`;
        document.getElementById("game-container").appendChild(this.element);

        // listen for bomb drop button
        document.addEventListener("keydown", (event) => {
            if (event.key === " ") { // drop bomb with space
                this.dropBomb();
            }
        });

        // listen for direction controls
        this.left = false;
        this.right = false;
        this.up = false;
        this.down = false;
        // bind move() and stop() to this instance (instead of document) with arrow functions
        document.addEventListener('keydown', (event) => this.move(event));
        document.addEventListener('keyup', (event) => this.stop(event));
        this.isMoving = false;

        this.invulnerability();
    };

    invulnerability() {
        let countNow = timedCount;
        this.vulnerable = false;
        this.element.classList.add("invulnerable");

        const timedInvulnerability = new Timer(() => {
            this.vulnerable = true;
            this.element.classList.remove("invulnerable");
            timedEvents.delete(`invulnerability${countNow}`)
        }, 2000);

        timedEvents.set(`invulnerability${countNow}`, timedInvulnerability)
        timedCount++;
    }

    dropBomb() {
        const row = Math.floor((this.y + this.size / 2) / gridStep);
        const col = Math.floor((this.x + this.size / 2) / gridStep);

        if (this.alive && this.bombAmount > 0 && (!levelMap[row][col] || levelMap[row][col] === "player")) {

            // find from bombPool, start explode method
            const bomb = bombsPool.find((b) => !b.active);
            bomb.drop(row, col, this.bombPower, 'player');

            this.bombAmount--;

            let countNow = timedCount;
            const timedBombsBack = new Timer(() => {
                this.bombAmount++;
                timedEvents.delete(`bombsback${countNow}`);
            }, bombTime);
            timedEvents.set(`bombsback${countNow}`, timedBombsBack)
            timedCount++;
        };
    };

    // Handle sprite direction change based on movement
    updateSpriteDirection(key) {
        if (this.alive) {
            if (key == 'ArrowLeft') {
                this.element.classList.add('left');
            }
            if (key == 'ArrowRight') {
                this.element.classList.remove('left');
            }
        }
    }

    move(event) {
        switch (event.key) {
            case "ArrowLeft":
                this.left = true;
                break;
            case "ArrowRight":
                this.right = true;
                break;
            case "ArrowUp":
                this.up = true;
                break;
            case "ArrowDown":
                this.down = true;
                break;
        };
        this.updateSpriteDirection(event.key); // Update the sprite if player moves left or right    
    };

    stop(event) {
        switch (event.key) {
            case "ArrowLeft":
                this.left = false;
                break;
            case "ArrowRight":
                this.right = false;
                break;
            case "ArrowUp":
                this.up = false;
                break;
            case "ArrowDown":
                this.down = false;
                break;
        };
    };

    die() {
        this.element.classList.add('dead');

        this.alive = false;
        this.lives--;
        updateLivesInfo(this.lives);

        // Stop walking sound when player dies
        walkingSound.pause();
        walkingSound.currentTime = 0;
        levelMap[0][0] = 'player';  // make sure enemies don't walk over player

        const countNow = timedCount;
        const timedResurrection = new Timer(() => {
            if (this.lives > 0) {
                this.x = this.startX;
                this.y = this.startY;
                this.element.style.transform = `translate(${this.x}px, ${this.y}px)`;
                this.element.classList.remove('dead');
                this.alive = true;
                this.invulnerability();
            } else {
                const gameOverMenu = document.getElementById("game-over-menu");
                const gifs = ["images/loser1.gif", "images/loser2.gif"];
                const randomGif = gifs[Math.floor(Math.random() * gifs.length)];
                gameOverMenu.style.background = `rgba(0, 0, 0, 0.8) url("${randomGif}") no-repeat center center`;
                gameOverMenu.style.backgroundSize = "cover";
                gameOverMenu.style.display = "block";

                levelMusic.forEach(track => {
                    track.pause();
                    track.currentTime = 0;
                });
                enemies.forEach(enemy => {
                    enemy.enemyWalking.pause();
                    enemy.enemyWalking.currentTime = 0;
                });
                setGameLost(); // Stop game loop updates

                if (randomGif === "images/loser1.gif") {
                    gameLost1.play(); // sad-trombone for loser1.gif
                } else {
                    gameLost2.play(); // sinister-laugh for loser2.gif
                }
            };
            timedEvents.delete(`resurrection${countNow}`)
        }, 2000);

        // Block enemies for 2 seconds after resurrection
        const timedEnemyBlock = new Timer(() => {
            if (this.lives > 0) {
                levelMap[0][0] = '';
            }
            timedEvents.delete(`enemyBlock${countNow}`)
        }, 4000);

        timedEvents.set(`resurrection${countNow}`, timedResurrection)
        timedEvents.set(`enemyBlock${countNow}`, timedEnemyBlock)
        timedCount++;
    };

    movePlayer(deltaTime) {

        if (this.alive) {

            // diagonal movement slowdown factor
            let slowDown = 1;
            if ((this.left || this.right) && (this.up || this.down)) {
                slowDown = 0.707;
            };

            // normalize speed for diagonal movement and different framerates
            let moveDistance = this.speed * slowDown * deltaTime;

            // calculate next position
            let newX = this.x;
            let newY = this.y;
            if (this.left) newX -= moveDistance;
            if (this.right) newX += moveDistance;
            if (this.up) newY -= moveDistance;
            if (this.down) newY += moveDistance;

            // solid wall collisions
            const collidingWalls = [];
            for (const wall of solidWalls) {
                if (wall.checkCollision(newX, newY, this.size, slowDown).toString() != [newX, newY].toString()) {
                    collidingWalls.push(wall);
                    if (collidingWalls.length == 1) break; // Can't collide with more than one solid wall
                };
            };

            // weak wall collisions
            for (const wall of weakWalls.values()) {
                if (wall.checkCollision(newX, newY, this.size, slowDown).toString() != [newX, newY].toString()) {
                    collidingWalls.push(wall);
                    if (collidingWalls.length === 3) break; // Can't collide with more than three walls
                };
            };

            // adjust next coordinates based on collisions to walls
            for (const wall of collidingWalls) {
                [newX, newY] = wall.checkCollision(newX, newY, this.size, slowDown, collidingWalls.length);
            };

            // bomb collisions
            const collidingBombs = [];
            for (const bomb of bombs.values()) {
                if (bomb.checkCollision(newX, newY, this.size).toString() != [newX, newY].toString()) {
                    collidingBombs.push(bomb);
                } else {
                    // erase owner when player no longer on top of bomb
                    bomb.owner = '';
                };
            };

            // adjust next coordinates based on collisions to bombs
            for (const bomb of collidingBombs) {
                // No collision if bomb has owner
                if (!bomb.owner) {
                    [newX, newY] = bomb.checkCollision(newX, newY, this.size);
                };
            };

            // set coordinates based on possible collisions to area boundaries
            this.x = Math.max(0, Math.min(newX, bounds.width - this.size));
            this.y = Math.max(0, Math.min(newY, bounds.height - this.size));

            // apply movement
            this.element.style.transform = `translate(${this.x}px, ${this.y}px)`;

            // Walking sound logic
            const wasMoving = this.isMoving;
            this.isMoving = this.left || this.right || this.up || this.down;
            if (this.isMoving && !wasMoving) {
                walkingSound.play();
            } else if (!this.isMoving && wasMoving) {
                walkingSound.pause();
                walkingSound.currentTime = 0;
            }

            // Fatal, power-up and finish collisions after movement 

            let playerBounds = this.element.getBoundingClientRect();

            if (this.vulnerable) {

                // flames hit
                for (const flame of flames.values()) {
                    if (checkHit(playerBounds, flame)) {
                        playerBombDeath.play();
                        this.die();
                        break;
                    };
                };

                // enemies hit
                for (const enemy of enemies.values()) {
                    if (enemy.alive && checkHit(playerBounds, enemy.element)) {
                        if (window.deathSound === 0) {
                            playerDeath.play();
                            window.deathSound = 1;
                        } else {
                            playerDeath2.play();
                            window.deathSound = 0;
                        }
                        this.die();
                        break;
                    };
                };
            }

            // power-ups hit
            for (const pow of powerups.values()) {
                if (checkHit(playerBounds, pow.element)) {
                    if (pow.powerType === "bomb") {
                        this.bombAmount++;
                        pow.pickUp();
                    }
                    if (pow.powerType === "flame") {
                        this.bombPower++;
                        pow.pickUp();
                    }
                    break;
                };
            };

            // finish hit
            if (finish.active && finish.checkCollision(newX, newY, this.size)) {
                this.alive = false;
                walkingSound.pause();
                walkingSound.currentTime = 0;
                levelMusic.forEach(track => {
                    track.pause();
                    track.currentTime = 0;
                });
                finishLevel.play();
                toggleFinished();

               // this.element.style.backgroundImage = `url('images/finish.svg')`

                // Trigger the finish animation
                playFinishAnimation();

                const timedNextLevel = new Timer(() => {
                    nextLevel();
                    timedEvents.delete(`finishingTheLevel`);
                }, 4000);
                timedEvents.set(`finishingTheLevel`, timedNextLevel);
                timedCount++;
                finish.active = false;
            };
        };
    };
};

function checkHit(playerBounds, other) {
    const otherBounds = other.getBoundingClientRect();

    // No hit (false) if player is safely outside on at least one side
    return !(playerBounds.right - mult * 10 < otherBounds.left ||
        playerBounds.left + mult * 10 > otherBounds.right ||
        playerBounds.bottom - mult * 10 < otherBounds.top ||
        playerBounds.top + mult * 10 > otherBounds.bottom);
};


function playFinishAnimation() {
    const finishImages = [
        'images/finish8.png',
        'images/finish7.png',
        'images/finish6.png',
        'images/finish5.png',
        'images/finish4.png',
        'images/finish3.png',
        'images/finish2.png',
        'images/finish1.png',
    ];

    let currentImageIndex = 0;
    const totalImages = finishImages.length;


    // Set initial image for the animation
    finish.element.style.backgroundImage = `url('${finishImages[currentImageIndex]}')`;

      // Set a timeout for how long you want the animation to run (e.g., 4 seconds)
      const animationDuration = 6000; // 6 seconds for the animation
      const startTime = Date.now(); // Record the start time

    // Create a timer to switch images in the animation sequence
    const animationInterval = setInterval(() => {
        currentImageIndex++;

        if (currentImageIndex < totalImages) {
            finish.element.style.backgroundImage = `url('${finishImages[currentImageIndex]}')`;
        } else  {
            // Reset back to the first image to loop
            currentImageIndex = 0;
            finish.element.style.backgroundImage = `url('${finishImages[currentImageIndex]}')`;
        }

        // If animation runs for 6 seconds, stop it and revert to the static finish image
        if (Date.now() - startTime >= animationDuration) {
            clearInterval(animationInterval);  // Stop the animation
            finish.element.style.backgroundImage = `url('images/finishgrey.svg')`;  // Revert back to the static image
            // Optionally, call nextLevel() here or any other logic to proceed to the next level
             //nextLevel();
        }
    }, 100); // Change image every 100ms
}