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
        return [<Header />, "choose a website"];
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

ReactDOM.render(
    <App />,
    document.getElementById('root'),
);
