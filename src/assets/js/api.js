class APIClient {
    constructor() {
        this.base = '/api';
        this.idTracker = new UsedIDTracker();

        this._isFetchingLevels = false;
        this._isSamplingListing = false;
    }

    async levels() {
        if (this._isFetchingLevels) {
            return false;
        }
        this._isFetchingLevels = true;
        try {
            const requestObject = {
                seenIDs: this.idTracker.seenIDs(),
            };
            const data = await this._postObject(this.base + '/levels', requestObject);
            return data.map((x) => {
                return new APILevel(x['website_name'], x['category_name'], x['id'], x['count']);
            });
        } finally {
            this._isFetchingLevels = false;
        }
    }

    async sampleListing(levelID) {
        if (this._isSamplingListing) {
            return false;
        }
        this._isSamplingListing = true;
        try {
            const requestObject = {
                seenIDs: this.idTracker.seenIDs(),
                level: levelID,
            };
            const data = await this._postObject(this.base + '/sample', requestObject);
            return new APIListing(data.id, data.title, data.price, data.imageURL);
        } finally {
            this._isSamplingListing = false;
        }
    }

    async _postObject(url, object) {
        return await this._getResult(fetch(url, {
            method: 'POST',
            cache: 'no-cache',
            headers: {
                'content-type': 'application/json',
            },
            body: JSON.stringify(object),
        }));
    }

    async _getResult(respPromise) {
        let response;
        try {
            response = await respPromise;
        } catch (e) {
            if (e.toString().includes('Failed to fetch')) {
                throw APIError('A network connection error was encountered. Please try again.');
            }
            throw e;
        }

        const data = await response.json();
        if (data.error) {
            throw new APIError(data.error);
        } else {
            return data.data;
        }
    }
}

class APILevel {
    constructor(website, category, id, count) {
        this.website = website;
        this.category = category;
        this.id = id;
        this.count = count;
    }
}

class APIListing {
    constructor(id, title, price, imageURL) {
        this.id = id;
        this.title = title;
        this.price = price;
        this.imageURL = imageURL;
    }
}

class APIError {
    constructor(msg) {
        this.msg = msg;
    }

    toString() {
        return this.msg;
    }
}

class UsedIDTracker {
    constructor() {
        this._seenIDs = JSON.parse(localStorage.seenIDs || '[]');
    }

    seenIDs() {
        return this._seenIDs;
    }

    add(id) {
        this._seenIDs.push(id);
        localStorage.seenIDs = JSON.stringify(this.seenIDs());
    }
}
