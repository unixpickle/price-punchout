class APIClient {
    constructor() {
        this.base = '/api';
    }

    async levels() {
        const data = await this._getResult(fetch(this.base + '/levels'));
        return data.map((x) => {
            return new APILevel(x['website_name'], x['category_name'], x['id'], x['last_seen']);
        });
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
            throw APIError(data.error);
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

class APIError {
    constructor(msg) {
        this.msg = msg;
    }

    toString() {
        return this.msg;
    }
}