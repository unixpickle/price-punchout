const client = new APIClient();

class App extends React.Component {
    constructor() {
        super()
        this.state = {
            page: 'loadingLevels',
            error: null,
            levels: null,
            levelWebsite: null,
            selectedLevel: null,
            numPlayers: null,
            currentListing: null,
            currentGuessValue: null,
            currentGuesses: null,
            roundResults: null,
        }
    }

    render() {
        // Flow of pages looks like:
        //
        //    loadingLevels => levelWebsite => levelCategory => levelPlayers
        // => loadingListing => guessing | noListings => guesses => scoreboard
        //

        if (this.state.page === 'loadingLevels') {
            return this.renderLoadingLevels();
        } else if (this.state.page === 'error') {
            return this.renderError();
        } else if (this.state.page === 'levelWebsite') {
            return this.renderLevelWebsite();
        } else if (this.state.page === 'levelCategory') {
            return this.renderLevelCategory();
        } else if (this.state.page === 'levelPlayers') {
            return this.renderLevelPlayers();
        } else if (this.state.page === 'loadingListing') {
            return this.renderLoadingListing();
        } else if (this.state.page === 'noListings') {
            return this.renderNoListings();
        } else if (this.state.page === 'guessing') {
            return this.renderGuessing();
        } else if (this.state.page === 'guesses') {
            return this.renderGuesses();
        } else if (this.state.page === 'scoreboard') {
            return this.renderScoreboard();
        }

        return <Header />;
    }

    showError(e) {
        this.setState({
            page: 'error',
            error: e,
        });
    }

    newGame() {
        this.setState({ page: 'loadingLevels' });
    }

    renderLoadingLevels() {
        client.levels().then((levels) => {
            if (this.state.page === 'loadingLevels') {
                this.setState({
                    page: 'levelWebsite',
                    levels: levels,
                });
            }
        }).catch((e) => {
            this.showError(e.toString());
        })
        return [<Header />, <Loader />];
    }

    renderError() {
        return [<Header onNewGame={() => this.newGame()} />, <Error message={this.state.error} />];
    }

    renderLevelWebsite() {
        return [
            <Header />,
            <WebsitePicker levels={this.state.levels} onChoice={(website) => {
                this.setState({
                    page: 'levelCategory',
                    levelWebsite: website,
                });
            }} />,
        ];
    }

    renderLevelCategory() {
        return [
            <Header onNewGame={() => this.newGame()} />,
            <CategoryPicker
                levels={this.state.levels}
                website={this.state.levelWebsite}
                onChoice={(level) => {
                    this.setState({
                        page: 'levelPlayers',
                        selectedLevel: level,
                        numPlayers: 2,
                    });
                }}
                onBack={() => this.setState({ page: 'levelWebsite' })} />
        ];
    }

    renderLevelPlayers() {
        return [
            <Header onNewGame={() => this.newGame()} />,
            <PlayersPicker
                onChoice={(count) => {
                    this.setState({
                        page: 'loadingListing',
                        numPlayers: count,
                        roundResults: [],
                    })
                }}
                onBack={() => this.setState({ page: 'levelCategory' })} />
        ];
    }

    renderLoadingListing() {
        client.sampleListing(this.state.selectedLevel.id).then((listing) => {
            if (this.state.page === 'loadingListing') {
                if (listing.title === null) {
                    this.setState({ page: 'noListings' });
                    return;
                }
                this.setState({
                    page: 'guessing',
                    currentListing: listing,
                    currentGuesses: [],
                    currentGuessValue: '',
                });
            }
        }).catch((e) => {
            this.showError(e.toString());
        });
        return [<Header onNewGame={() => this.newGame()} />, <Loader />];
    }

    renderNoListings() {
        if (this.state.roundResults.length > 0) {
            return [
                <Header onNewGame={() => this.newGame()} />,
                <Scoreboard
                    roundResults={this.state.roundResults}
                    done={true}
                    onNewGame={() => this.setState({ page: 'loadingLevels' })} />
            ];
        } else {
            this.showError('There are no more items in this level');
        }
    }

    renderGuessing() {
        const player = 1 + this.state.currentGuesses.length;
        return [
            <Header onNewGame={() => this.newGame()} />,
            <GuessPicker
                player={player}
                listing={this.state.currentListing}
                value={this.state.currentGuessValue}
                onChange={(e) => this.setState({ currentGuessValue: e.target.value })}
                onSkip={() => {
                    client.idTracker.add(this.state.currentListing.id);
                    this.setState({ page: 'loadingListing' });
                }}
                onChoice={(guess) => {
                    const newGuesses = this.state.currentGuesses.concat([guess]);
                    if (player === this.state.numPlayers) {
                        client.idTracker.add(this.state.currentListing.id);
                        const result = new RoundResult(this.state.currentListing, newGuesses);
                        this.setState({
                            page: 'guesses',
                            currentGuessValue: '',
                            currentGuesses: [],
                            roundResults: this.state.roundResults.concat([result]),
                        });
                    } else {
                        this.setState({
                            currentGuessValue: '',
                            currentGuesses: newGuesses,
                        });
                    }
                }} />
        ];
    }

    renderGuesses() {
        return [
            <Header onNewGame={() => this.newGame()} />,
            <Guesses
                listing={this.state.currentListing}
                lastResults={this.state.roundResults[this.state.roundResults.length - 1]}
                onNext={() => this.setState({ page: 'scoreboard' })} />
        ];
    }

    renderScoreboard() {
        return [
            <Header onNewGame={() => this.newGame()} />,
            <Scoreboard
                roundResults={this.state.roundResults}
                done={false}
                onNext={() => this.setState({ page: 'loadingListing' })} />
        ];
    }
}

function Header(props) {
    return <div id="logo-header">
        {props.onNewGame ? <button id="new-game" onClick={props.onNewGame}>New Game</button> : null}
    </div>;
}

function Loader() {
    return <div class="loader"></div>;
}

function Error(props) {
    return <div class="content-pane error">{props.message}</div>
}

function WebsitePicker(props) {
    const websites = {};
    props.levels.forEach((level) => {
        websites[level.website] = true;
    });
    const items = Object.keys(websites).sort().map((website) => (
        <li class="choice-list-item" onClick={() => props.onChoice(website)}>
            <img class="choice-list-item-icon" src={websiteIcon(website)}></img>
            <label class="choice-list-item-text">
                <p>{website}</p>
            </label>
        </li>
    ));
    return <div class="content-pane">
        <div class="content-pane-header">
            <h1>Select website</h1>
        </div>
        <div class="choice-list-container">
            <ul class="choice-list">{items}</ul>
        </div>
    </div>;
}

function websiteIcon(website) {
    if (website === 'Amazon') {
        return '/svg/amazon_box.svg';
    } else if (website === 'Target') {
        return '/svg/target.svg';
    } else {
        return '/svg/unknown.svg';
    }
}

function CategoryPicker(props) {
    const levels = props.levels.filter((x) => x.website == props.website);
    const items = levels.sort().map((level) => (
        <li class="choice-list-item" onClick={() => props.onChoice(level)}>
            <img class="choice-list-item-icon" src={levelIcon(level)}></img>
            <div class="choice-list-item-text">
                <p>{level.category}</p>
            </div>
        </li>
    ));
    return <div class="content-pane">
        <div class="content-pane-header">
            <button class="back-button" onClick={props.onBack}>Back</button>
            <h1>Select category</h1>
        </div>
        <div class="choice-list-container">
            <ul class="choice-list">{items}</ul>
        </div>
    </div>;
}

function levelIcon(level) {
    if (level.id === 'amazon-all') {
        return '/svg/amazon_box.svg';
    } else if (level.id === 'amazon-if') {
        return '/svg/treasure_chest.svg';
    } else if (level.id === 'amazon-thi') {
        return '/svg/wrench.svg';
    } else if (level.id === 'target-clothes') {
        return '/svg/t_shirt.svg';
    } else if (level.id === 'target-sports-outdoors') {
        return '/svg/camping.svg';
    } else if (level.id === 'target-all') {
        return '/svg/target.svg';
    } else {
        return '/svg/unknown.svg';
    }
}

function PlayersPicker(props) {
    const [numPlayers, setNumPlayers] = React.useState('2');
    const parsed = parseInt(numPlayers);
    const valid = (
        !isNaN(parsed) &&
        parsed.toString() == numPlayers.trim() &&
        parsed > 1 &&
        parsed < 100
    );
    return <div class="content-pane">
        <div class="content-pane-header">
            <button class="back-button" onClick={props.onBack}>Back</button>
            <h1>How many players?</h1>
        </div>
        <input
            class={valid ? "player-count-input" : "player-count-input player-count-input-invalid"}
            value={numPlayers}
            type="number"
            onKeyUp={(e) => {
                if (valid && e.key === 'Enter') {
                    props.onChoice(parsed);
                }
            }}
            onChange={(e) => setNumPlayers(e.target.value)} />
        <button
            class={valid ? "ok-button" : "ok-button ok-button-disabled"}
            onClick={() => {
                if (valid) {
                    props.onChoice(parsed);
                }
            }}>Play!</button>
    </div>;
}

function GuessPicker(props) {
    const price = props.value;

    let trimmed = price.trim();
    if (/[0-9]+\.[1-9]0/.test(price)) {
        trimmed = price.substr(0, price.length - 1);
    } else if (/[0-9]+\.00?/.test(price)) {
        trimmed = price.substr(0, price.length - 3);
    }

    const parsed = parseFloat(trimmed);
    const valid = (
        !isNaN(parsed) &&
        parsed.toString() == trimmed &&
        parsed > 0
    );

    return <div class="content-pane">
        <div class="content-pane-header">
            <h1>Guess for Player {props.player}</h1>
        </div>
        <div class="product-listing">
            <div class="product-listing-thumbnail-container">
                <img class="product-listing-thumbnail" src={props.listing.imageURL} />
            </div>
            <p class="product-listing-text">{props.listing.title}</p>
        </div>
        <input
            class={"product-price-guess " + ((valid || !price) ? "" : "product-price-guess-invalid")}
            value={price}
            type="number"
            autoFocus
            onKeyUp={(e) => {
                if (valid && e.key === 'Enter') {
                    props.onChoice(parsed);
                }
            }}
            placeholder={"Guess for Player " + props.player}
            onChange={props.onChange} />
        <button
            class={valid ? "ok-button" : "ok-button ok-button-disabled"}
            onClick={() => {
                if (valid) {
                    props.onChoice(parsed);
                }
            }}>Submit</button>
        <div class="skip-button-container">
            <button
                class="skip-button"
                onClick={props.onSkip}>Skip</button>
        </div>
    </div>;
}

function Guesses(props) {
    const results = props.lastResults;

    const rows = results.guesses.map((x, i) => {
        return <tr>
            <td>
                <WinnerStatus
                    player={i}
                    result={props.lastResults} />
            </td>
            <td>
                Player {i + 1}
            </td>
            <td>${x}</td>
        </tr>;
    });

    return <div class="content-pane">
        <div class="content-pane-header">
            <h1>Guesses</h1>
        </div>
        <div class="product-listing">
            <div class="product-listing-thumbnail-container">
                <img class="product-listing-thumbnail" src={props.listing.imageURL} />
            </div>
            <p class="product-listing-text">{props.listing.title}</p>
        </div>
        <label class="product-price-guesses-label">Guesses:</label>
        <table class="guesses-table">
            {rows}
        </table>
        <div class="product-price-answer">
            {"$" + (props.listing.price / 100).toFixed(2)}
        </div>
        <button
            class="ok-button"
            onClick={props.onNext}>Next</button>
    </div>;
}

function Scoreboard(props) {
    const results = props.roundResults;
    const numPlayers = results[0].guesses.length;
    const scores = [];
    for (let i = 0; i < numPlayers; ++i) {
        scores[i] = 0;
    }
    results.forEach((result) => {
        const winners = result.winners();
        winners.forEach((i) => {
            scores[i] += 1 / winners.length;
        });
    })

    const rows = scores.map((x, i) => {
        return <tr>
            <td>Player {i + 1}</td>
            <td>{x} points</td>
        </tr>;
    });

    const nextButton = (
        <button
            class="ok-button"
            onClick={props.onNext}>Next</button>
    );
    const doneButton = (
        <button
            class="ok-button"
            onClick={props.onNewGame}>New Game</button>
    );

    return <div class="content-pane">
        <div class="content-pane-header">
            <h1>Scoreboard</h1>
        </div>
        {props.done ? <p class="error">There are no more items in this level</p> : null}
        <table class="scoreboard-table">
            {rows}
        </table>
        {props.done ? doneButton : nextButton}
    </div>;
}

function WinnerStatus(props) {
    const player = props.player;
    const result = props.result;
    const won = result.winners().includes(player);
    return <div class={"winner-status winner-status-" + (won ? "winner" : "loser")}></div>;
}

class RoundResult {
    constructor(listing, guesses) {
        this.listing = listing;
        this.guesses = guesses;
    }

    winners() {
        let bestGuess = Infinity;
        let indices = [];
        this.guesses.forEach((x, i) => {
            const err = Math.abs(x - this.listing.price / 100);
            const bestErr = Math.abs(bestGuess - this.listing.price / 100);
            if (err < bestErr) {
                bestGuess = x;
                indices = [i];
            } else if (err === bestErr) {
                indices.push(i);
            }
        });
        return indices;
    }
}

ReactDOM.render(
    <App />,
    document.getElementById('root'),
);

function setupBackground() {
    const bg = document.getElementById('background');
    const size = Math.max(window.innerWidth, window.innerHeight);
    bg.style.fontSize = size.toFixed(2) + 'px';
}

setupBackground();
window.addEventListener('resize', setupBackground);
