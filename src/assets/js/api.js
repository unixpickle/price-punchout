class APIClient {
    constructor() {
        this.base = '/api';
        this.idTracker = new UsedIDTracker();
    }

    async levels() {
        const data = await this._getResult(fetch(this.base + '/levels'));
        return data.map((x) => {
            return new APILevel(x['website_name'], x['category_name'], x['id'], x['last_seen']);
        });
    }

    async sampleListing(levelID) {
        const requestObject = {
            seenIDs: this.idTracker.seenIDs(),
            level: levelID,
        };
        const data = await this._getResult(fetch(this.base + '/sample?', {
            method: 'POST',
            cache: 'no-cache',
            headers: {
                'content-type': 'application/json',
            },
            body: JSON.stringify(requestObject),
        }));
        return new APIListing(data.id, data.title, data.price, data.imageURL);
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
    constructor(website, category, id, lastSeen) {
        this.website = website;
        this.category = category;
        this.id = id;
        this.lastSeen = lastSeen;
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
