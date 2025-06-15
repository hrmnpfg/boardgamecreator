FROM rust:latest

RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g yarn

WORKDIR /app

COPY . .

RUN yarn install

RUN yarn build

EXPOSE 8000

CMD ["node", "main.mjs"]
