const client = new APIClient();

function App() {
    //    loadingLevels => levelWebsite => levelCategory => levelPlayers
    // => loadingListing => guessing | noListings => scoreboard
    const [page, setPage] = React.useState('loadingLevels');
    const [error, setError] = React.useState(null);
    const [levels, setLevels] = React.useState(null);

    const [levelWebsite, setLevelWebsite] = React.useState(null);
    const [selectedLevel, setSelectedLevel] = React.useState(null);
    const [numPlayers, setNumPlayers] = React.useState(2);
    const [currentListing, setCurrentListing] = React.useState(null);
    const [currentGuessValue, setCurrentGuessValue] = React.useState('');
    const [currentGuesses, setCurrentGuesses] = React.useState(null);
    const [roundResults, setRoundResults] = React.useState([]);

    if (page === 'loadingLevels') {
        client.levels().then((levels) => {
            setLevels(levels);
            setPage('levelWebsite');
        }).catch((e) => {
            setError(e.toString());
            setPage('error');
        })
        return [<Header />, <Loader />];
    } else if (page === 'error') {
        return [<Header />, <Error message={error} />];
    } else if (page === 'levelWebsite') {
        return [
            <Header />,
            <WebsitePicker levels={levels} onChoice={(website) => {
                setLevelWebsite(website);
                setPage('levelCategory');
            }} />,
        ];
    } else if (page === 'levelCategory') {
        return [
            <Header />,
            <CategoryPicker
                levels={levels}
                website={levelWebsite}
                onChoice={(level) => {
                    setSelectedLevel(level);
                    setPage('levelPlayers');
                }}
                onBack={() => setPage('levelWebsite')} />
        ]
    } else if (page === 'levelPlayers') {
        return [
            <Header />,
            <PlayersPicker
                onChoice={(count) => {
                    setNumPlayers(count);
                    setRoundResults([]);
                    setPage('loadingListing');
                }}
                onBack={() => setPage('levelCategory')} />
        ]
    } else if (page === 'loadingListing') {
        client.sampleListing(selectedLevel.id).then((listing) => {
            if (listing.title === null) {
                setPage('noListings');
                return;
            }
            setCurrentListing(listing);
            setCurrentGuesses([]);
            setPage('guessing');
        }).catch((e) => {
            setError(e.toString());
            setPage('error');
        })
        return [<Header />, <Loader />];
    } else if (page === 'noListings') {
        // TODO: show scoreboard here.
        return [<Header />, <Error message="No listings remain in this category." />]
    } else if (page === 'guessing') {
        const player = 1 + currentGuesses.length;
        return [
            <Header />,
            <GuessPicker
                player={player}
                listing={currentListing}
                value={currentGuessValue}
                onChange={(e) => setCurrentGuessValue(e.target.value)}
                onChoice={() => {
                    const newGuesses = currentGuesses.concat([currentGuessValue]);
                    setCurrentGuesses(newGuesses);
                    setCurrentGuessValue('');
                    if (player === numPlayers) {
                        client.idTracker.add(currentListing.id);
                        const result = new RoundResult(currentListing, newGuesses);
                        setRoundResults(roundResults.concat([result]));
                        setPage('scoreboard');
                    }
                }} />
        ];
    } else if (page === 'scoreboard') {
        return [
            <Header />,
            <Scoreboard
                roundResults={roundResults}
                onNext={() => {
                    setPage('loadingListing');
                }} />
        ];
    }

    return <Header />;
}

function Header() {
    return <div id="logo-header"></div>;
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
    if (level.category === 'Interesting Finds') {
        return '/svg/treasure_chest.svg';
    } else if (level.category == 'Tools and Home Improvement') {
        return '/svg/calculator.svg';
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
    const parsed = parseFloat(price);
    const valid = (
        !isNaN(parsed) &&
        parsed.toString() == price.trim() &&
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
    </div>;
}

function Scoreboard(props) {
    const results = props.roundResults;
    const numPlayers = results[0].guesses.length;
    console.log('num players', numPlayers);
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
            <td>Player {i}</td>
            <td>{x}</td>
        </tr>;
    });

    return <div class="content-pane">
        <div class="content-pane-header">
            <h1>Scoreboard</h1>
        </div>
        <table class="scoreboard-table">
            {rows}
        </table>
        <button
            class="ok-button"
            onClick={props.onNext}>Next</button>
    </div>;
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
