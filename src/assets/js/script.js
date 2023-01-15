const client = new APIClient();

function App() {
    //    loadingLevels => levelWebsite => levelCategory => levelPlayers
    // => loadingListing => showingListing => guessing => results
    const [page, setPage] = React.useState('loadingLevels');
    const [error, setError] = React.useState(null);
    const [levels, setLevels] = React.useState(null);

    const [levelWebsite, setLevelWebsite] = React.useState(null);
    const [selectedLevel, setSelectedLevel] = React.useState(null);
    const [numPlayers, setNumPlayers] = React.useState(2);
    const [currentListing, setCurrentListing] = React.useState(null);
    const [currentGuesses, setCurrentGuesses] = React.useState(null);
    const [scoreboard, setScoreboard] = React.useState(null);

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
    }

    return Header();
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
        <h1>Select website</h1>
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
            <label class="choice-list-item-text">
                <p>{level.category}</p>
            </label>
        </li>
    ));
    return <div class="content-pane">
        <button class="back-button" onClick={props.onBack}>Back</button>
        <h1>Select category</h1>
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

ReactDOM.render(
    <App />,
    document.getElementById('root'),
);
