
exports.p51 = {
	series_name: 'three-1-',
	params: {
		max_supply: '200',
		enforce_unique_args: true,
		mint: [
			'seed'
		],
		owner: [
			'speed'
		],
		packages: [
			'p5@1.3.1'
		]
	},
	src: `@params
	{
		packages: ['p5@1.3.1'],
		max_supply: '200',
		mint: {
			seed: {
				default: 0,
				type: 'int',
			},
		},
		owner: {
			speed: {
				default: 30,
				type: 'int',
			},
			blur: {
				default: 2,
				type: 'float',
			},
			opacity: {
				default: 0.1,
				type: 'float',
			},
			foreground: {
				default: [255, 127, 0],
				type: 'color-arr',
			},
			background: {
				default: [255, 223, 223],
				type: 'color-arr',
			},
		}
	}
	@params
	
	@css
	body { margin: 0; overflow: hidden; }
	@css
	
	@js
	
	// these vars are loaded by token params
	const seed = {{seed}}
	const speed = {{speed}}
	const blur = {{blur}}
	const opacity = {{opacity}}
	const foreground = {{foreground}}
	const background = {{background}}
	
	// processing vars
	let width;
	let height;
	let w;
	let columns;
	let rows;
	let board;
	let next;
	
	function setup() {
		let width = window.innerWidth
		let height = window.innerHeight
		randomSeed(seed);
		frameRate(speed);
		createCanvas(width, height);
		rectMode(CENTER);
		w = width / 50;
		h = height / 50;
		// Calculate columns and rows
		columns = floor(width / w);
		rows = floor(height / h) + 1;
		// Wacky way to make a 2D array is JS
		board = new Array(columns);
		for (let i = 0; i < columns; i++) {
			board[i] = new Array(rows);
		}
		// Going to use multiple 2D arrays and swap them
		next = new Array(columns);
		for (i = 0; i < columns; i++) {
			next[i] = new Array(rows);
		}
		stroke(255, 0);
		strokeWeight(0.5);
		init();
	}
	
	function draw() {
		generate();
		for ( let i = 0; i < columns;i++) {
			for ( let j = 0; j < rows;j++) {
				push();
				translate((i+0.5) * w + blur * w/2, (j+0.5) * h + blur * h/2);
				scale(blur, blur);
				
				if ((board[i][j] == 1)) fill(foreground[0], foreground[1], foreground[2], 255 * opacity);
				else fill(background[0], background[1], background[2], 255 * opacity);
				rect(-w/2, -h/2, w, h);
	
				pop();
			}
		}
	}
	
	// reset board when mouse is pressed
	function mousePressed() {
		setup();
	}
	
	// Fill board randomly
	function init() {
		for (let i = 0; i < columns; i++) {
			for (let j = 0; j < rows; j++) {
			// Lining the edges with 0s
			if (i == 0 || j == 0 || i == columns-1 || j == rows-1) board[i][j] = 0;
			// Filling the rest randomly
			else board[i][j] = floor(random(2));
			next[i][j] = 0;
			}
		}
	}
	
	// The process of creating the new generation
	function generate() {
		// Loop through every spot in our 2D array and check spots neighbors
		for (let x = 1; x < columns - 1; x++) {
			for (let y = 1; y < rows - 1; y++) {
			// Add up all the states in a 3x3 surrounding grid
			let neighbors = 0;
			for (let i = -1; i <= 1; i++) {
				for (let j = -1; j <= 1; j++) {
				neighbors += board[x+i][y+j];
				}
			}
	
			// A little trick to subtract the current cell's state since
			// we added it in the above loop
			neighbors -= board[x][y];
			// Rules of Life
			if      ((board[x][y] == 1) && (neighbors <  2)) next[x][y] = 0;           // Loneliness
			else if ((board[x][y] == 1) && (neighbors >  3)) next[x][y] = 0;           // Overpopulation
			else if ((board[x][y] == 0) && (neighbors == 3)) next[x][y] = 1;           // Reproduction
			else                                             next[x][y] = board[x][y]; // Stasis
			}
		}
		// Swap!
		let temp = board;
		board = next;
		next = temp;
	}
	@js
		
`};
