const client = new APIClient();

function App() {
    //    loadingLevels => levelWebsite => levelCategory => levelPlayers
    // => loadingListing => showingListing => guessing => results
    const [page, setPage] = React.useState('loadingLevels');
    const [error, setError] = React.useState(null);
    const [levels, setLevels] = React.useState(null);

    const [levelWebsite, setLevelWebsite] = React.useState(null);
    const [levelCategory, setLevelCategory] = React.useState(null);
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
        return [<Header />, <WebsitePicker levels={levels} />];
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
        <li class="listing-item">
            <img class="listing-item-icon" src={websiteIcon(website)}></img>
            <label class="listing-item-name">{website}</label>
        </li>
    ));
    return <div class="content-pane">
        <h1>Select website</h1>
        <ul>{items}</ul>
    </div>;
}

function websiteIcon(website) {
    if (website === 'Amazon') {
        return '/svg/amazon_box.svg';
    } else {
        return '/svg/unknown.svg';
    }
}

ReactDOM.render(
    <App />,
    document.getElementById('root'),
);
