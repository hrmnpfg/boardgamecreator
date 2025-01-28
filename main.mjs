import { GameWrapper } from './index.js';
import http from 'http';
import express from 'express';
import {Server} from 'socket.io';
import cookieParser from 'cookie-parser';
import { v4 as v4 } from 'uuid';
import sqlite3 from 'sqlite3';
import multer from 'multer';
import path from 'path';
import { fileURLToPath } from 'url';
import * as fs from 'fs/promises';

class room
{
    name;
    room_ID;
    ID_first;
    ID_second;
    game;
    currentplayer;
    players;
    constructor(name,id)
    {
        this.name = name;
        this.room_ID = id;
        this.players = 0;
        this.currentplayer = "\nWhite";
        this.game = new GameWrapper();
        this.ended= false;
    }

}

var app = express();
var server = http.createServer(app);
var io = new Server(server);

app.set('view engine', 'ejs');
app.set('views', './res/views');


app.use(cookieParser());

app.use(express.urlencoded({ extended: true }));

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

app.use(express.static(path.join(__dirname, 'res/views/static')))

app.use(express.json());
const upload = multer({ dest: 'uploads/' })

let rooms = [];
let roomslist = [];

let db = new sqlite3.Database('./baza.db', (err) => {
  if (err) {
    console.error(err.message);
  }
  console.log('Connected to database.');
});

function receive(game){
    let ret = ""
    let recv = game.receive();
    while (recv != null) {
        ret = ret +"<br>"+ recv
        recv = game.receive();
    }
    return ret
}

function receive2(game){
    let ret = ""
    let recv = game.receive();
    while (recv != null) {
        ret = ret +"\n"+ recv
        recv = game.receive();
    }
    return ret
}

function receive3(game){
    let ret = ""
    let recv = game.receive();
    while (recv != null) {
        ret = ret + recv
        recv = game.receive();
    }
    return ret
}

function get_user(req, res){
    var username;
    var userid;
    if(!req.cookies.userid) {
        username = 'guest';
        userid = v4();
        res.cookie('username', username);
        res.cookie('userid', userid);
    }
    else {
        username = req.cookies.username;
        userid = req.cookies.userid;
    }
    return {username: username,userid: userid}
}

function get_games(){
    var sql = 'SELECT * FROM games';
    return new Promise((resolve,reject) =>{
        db.all(sql, [],(err,result)=>{
            if(err){
                return reject(err);
            }
            return resolve(result);
        });

    });
}

function get_game(id){
    var sql = 'SELECT * FROM games WHERE id = '+id;
    return new Promise((resolve,reject) =>{
        db.all(sql, [],(err,result)=>{
            if(err){
                return reject(err);
            }
            return resolve(result);
        });

    });
}

function insert_game(name, json){
    db.run("INSERT INTO games (name, json) VALUES (?, ?)", [name, json], (err) => {
        if(err){console.log(`sending to db failed ${err}`)}});
}

function get_pieces(){
    var sql = 'SELECT * FROM pieces';
    return new Promise((resolve,reject) =>{
        db.all(sql, [],(err,result)=>{
            if(err){
                return reject(err);
            }
            return resolve(result);
        });

    });
}

function get_piece(id){
    var sql = 'SELECT * FROM pieces WHERE id = '+id;
    return new Promise((resolve,reject) =>{
        db.all(sql, [],(err,result)=>{
            if(err){
                return reject(err);
            }
            return resolve(result);
        });

    });
}

function isstate(game, states){
    game.send("getstatus");
    let status = game.receive();
    return states.includes(status)
}

app.get( '/game2' ,(req,res) => {
    //assume that to be in /game game has to be in playing or finished state
    var id = req.query.room;
    var room = rooms[id];

    if (id == undefined || room == undefined || !isstate(room.game, ["playing", "finished"]) )
    {
        res.redirect("/");
    }

    let {username,userid} = get_user(req,res);
    room.game.send("show");
    let board = receive(room.game);
    room.game.send("getdimensions");
    let dimensions = receive2(room.game);
    let dims = dimensions.split(",");
    console.log(dimensions);
    room.game.send("show cementary white");
    let white_killed = receive2(room.game);
    room.game.send("show cementary black");
    let black_killed = receive2(room.game);
    res.render('game2',{room,userid,username,board, dims, white_killed, black_killed});
});

app.post('/game2', (req, res) => {
    let {username,userid} = get_user(req,res);
    var id = req.query.room;
    var room = rooms[id];
    let prompt = " ";

    if (typeof(room) == "undefined")
    {
        res.status(200).json({redirect: "/"});
        return
    }

    let ret = "not one of players";
    if (userid != room.ID_first && userid!=room.ID_second) {
        //blocking not playing users here
        res.status(200).json({resp: ret });
        return;
    }

    ret = "wrong player";


    if((room.currentplayer == "\nWhite" && userid==room.ID_first) || (room.currentplayer=="\nBlack" && room.ID_second && userid==room.ID_second)){

        let command = req.body.command;
        console.log("got command: ", command);
        if (command == "end"){
            room.ID_first = undefined;
            room.ID_second = undefined;
            res.redirect('/');
            return
        }

        room.game.send(command);
        ret = receive2(room.game);
        if (["\nBlack wins", "\nWhite wins", "\nIt's a draw"].includes(ret)){

            room.game.send("show");
            let board = receive(room.game);
            ret=ret.substring(1);
            res.status(200).json({resp: ret, board: board, ended: true });
            return;
        }
        if ( ret && (ret.startsWith("\nPick a move") || ret.startsWith("\nchange to")) ){
            console.log("fghjuki");
            prompt = ret;
            res.status(200).json({resp: ret, prompt:prompt });
            return;
        }
        console.log("avoided");
    }
    room.game.send("show");
    let board = receive(room.game);
    console.log("board", board);
    room.game.send("currentwhite");
    let player = receive2(room.game);
    room.game.send("show cementary white");
    let white_killed = receive2(room.game);
    room.game.send("show cementary black");
    let black_killed = receive2(room.game);
    room.currentplayer = player;
    res.status(200).json({resp: ret, board: board, prompt:prompt, wc: white_killed, bc: black_killed });
});

app.get( '/rooms', async (req, res) => {
    let {username,userid} = get_user(req,res);
    var id = req.query.room;
    var room = rooms[id];

    if (room == undefined){
        res.redirect("/");
        return;
    }

    room.game.send("getstatus"); //TODO check if this will do some bugs
    let state = room.game.receive();

    if ((state == "start" || state == "initboard" || state == "initpieces" || state == "placepieces") && userid == room.ID_first) {
        room.game.send("a");
        receive(room.game);
        res.redirect("/roomcreator?room="+id)
        return;
    }

    let games = await get_games();
    console.log("games: ", games);

    let filled = room.ID_first && room.ID_second
    if (state == "playing" || state == "finished" || (filled && room.ID_first!=userid && room.ID_second!=userid)){
        res.redirect("/game2?room="+id);
    }
    else res.render('rooms_n', {room, userid, games});

});

app.post('/rooms',async (req, res) => {
    let {username,userid} = get_user(req,res);
    var id = req.query.room;
    var room = rooms[id];
    console.log("eooom", room, typeof(room));
    if (typeof(room) != "undefined"){
        room.game.send("getstatus");
        let state = room.game.receive();
        console.log("saassa ", isstate(room.game, ["playing"]));
        if (state == "playing") {
            res.status(200).json({ redirect: `/game2?room=${id}` });
            return;
        }
        let ret = "not allowed";
        if (userid == room.ID_first || userid==room.ID_second) {

            let command = req.body.command;
            let split = command.split(' ');
            console.log("got command: ", command);
            console.log(split);
            if (split[0] == "load_json" && (state == "initboard" || state == "initpieces") && split.length == 2){
                let elem;
                if(state == "initboard"){
                    elem = await get_game(split[1]);
                }
                else{
                    elem = await get_piece(split[1]);
                }

                if(elem.length==0){
                    res.status(200).json({ resp: "no element with this id exists" });
                    return;
                }

                console.log("sending: ", split[0]+" "+elem[0]['json']);
                room.game.send(split[0]+" "+elem[0]['json']);


            }
            else{
                room.game.send(command);
            }

            ret = receive(room.game);

            room.game.send("getstatus");
            let state2 = room.game.receive();
            console.log("states ",state,state2);
            if (state2 == "initpieces" && state2!=state){
                let pieces = await get_pieces();
                console.log("pieces", pieces);
                res.status(200).json({ resp: ret, title: "Pieces", pieces: pieces });
                return;
            }

        }

        res.status(200).json({resp: ret });
    }
    else{
       // res.redirect("/");
        res.status(200).json({redirect: "/"});
    }
});

app.get('/roomcreator', async (req, res) => {
    let {username, userid} = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (room == undefined || !isstate(room.game, ["initboard"])){
        res.redirect("/");
        return;
    }
    let games = await get_games();

    res.render('roomcreator', { games, username, userid, id });
});

app.post('/roomcreator', async (req, res) => {
    let { username, userid } = get_user(req, res);
    let { choice } = req.body; // Wybrana gra lub "empty"

    var id = req.query.room;
    var room = rooms[id];

    if (choice === "empty") {
        room.game.send("default");
        console.log(receive(room.game))
        res.redirect(`/roomcreator/pieces?room=${id}`);
    }
    else {
        if (room && userid === room.ID_first) {
            console.log(choice)
            let game_json = (await get_game(choice))[0]["json"];
            console.log(game_json)
            room.game.sendJson(game_json);
            room.game.send(`load_json`);
            console.log(receive(room.game))
        }

        res.redirect(`/rooms?room=${id}`);
    }
});

app.post('/roomcreator/create', (req, res) => {
    let id = req.query.room;
    let room = rooms[id];

    if (!room || !isstate(room.game, ["initboard"])){
        res.redirect(`/`);
        return;
    }

    room.game.send("create");
    let a = receive(room.game);
    console.log("got: ",a);
    res.redirect('/boardcreator/?room='+id);
});

app.get('/boardcreator', async (req, res) => {
    let { username, userid } = get_user(req, res);
    let id = req.query.room;
    let room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["createboard"])){
        res.redirect(`/`);
        return;
    }

    room.game.send("show size");
    let dimensions = receive3(room.game);
    let dims = dimensions.split(",");
    room.game.send("show history_size");
    let history_size = receive3(room.game);
    room.game.send("show end_condition");
    let endcondition = receive3(room.game);
    room.game.send("show win_condition");
    let white = room.game.receive();
    let black = room.game.receive();
    room.game.receive(); // DO NOT REMOVE PLEASE WE WILL ALL DIE IF IT'S GONE
    let winconditions = [white, black]
    console.log(dims, history_size, endcondition, white, black);
    res.render('boardcreator', {dims, history_size, endcondition, winconditions, id});
});

app.post('/boardcreator', (req, res) =>{
    let { username, userid } = get_user(req, res);
    let x = req.body["boardsize_x"]; let y = req.body["boardsize_y"]; let hist = req.body["history_size"];
    var id = req.query.room;
    var room = rooms[id];

    if (room && userid === room.ID_first) {

        room.game.send(`set size ${x},${y}`);
        let a1 = receive(room.game);
        room.game.send(`set history_size ${hist}`);
        let a2 = receive(room.game);
        console.log("a1:",a1,"a2:",a2);
    }
    res.redirect("/boardcreator/?room="+id);
});

app.post('/boardcreator/createrule', (req, res) => {
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];
    if (!room || room.ID_first!=userid || !isstate(room.game, ["createboard"])){
        res.redirect(`/`);
        return;
    }

    let x = req.body["choice"]
    switch (x){
        case "endcondition": {
            room.game.send("set end_condition");
            break;
        }
        case "white": {
            room.game.send("set win_condition White");
            break;
        }
        case "black": {
            room.game.send("set win_condition Black");
            break;
        }
        case "continue":{
            room.game.send("continue");
            receive(room.game);
            room.game.send("getstatus");
            let stat = room.game.receive();
            if (stat == "initpieces"){
                res.redirect('/roomcreator/pieces/?room='+id)
                return;
            }
        }
        default: {
            return; //???
        }
    }
    receive(room.game);
    res.redirect('/rulecreator/?room='+id);
});

app.get('/rulecreator', (req, res) =>{
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["rulecreator"])){
        res.redirect(`/`);
        return;
    }

    room.game.send("list layers");
    let layers = receive3(room.game);
    room.game.send("list stack");
    let stack = receive3(room.game);
    room.game.send("list apicalls");
    let apicalls = receive2(room.game);
    room.game.send("list");
    let list = receive2(room.game);
    res.render('rulecreator', {layers, stack, apicalls, list,id});
});

app.post('/rulecreator', (req, res) => {
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["rulecreator"])){
        res.redirect(`/`);
        return;
    }

    let expr = req.body["exprtype"];
    room.game.send(expr);
    let x = receive(room.game);
    console.log(x);
    res.redirect("/rulecreator/?room="+id);
});

app.post('/rulecreator/value', (req, res) => {
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["rulecreator"])){
        res.redirect(`/`);
        return;
    }

    let expr = req.body["exprtype"];
    let args = req.body["args"];
    if(expr=="Void"){
        room.game.send("Void");
    }
    else{
        room.game.send(`${expr} ${args}`);
    }
    let x = receive(room.game);
    console.log(x);
    res.redirect("/rulecreator/?room="+id);
});

app.post('/rulecreator/button', (req, res) => {
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["rulecreator"])){
        res.redirect(`/`);
        return;
    }

    let command = req.body["choice"];
    room.game.send(command);
    receive(room.game);
    room.game.send("getstatus");
    let stat = room.game.receive();

    switch (stat){
        case "rulecreator":{
            res.redirect("/rulecreator/?room="+id); return;
        }
        case "createboard":{
            res.redirect("/boardcreator/?room="+id); return;
        }
        case "piececreator":{
            res.redirect("/piececreator/?room="+id); return;
        }
        case "movecreator":{
            res.redirect("/movecreator/?room="+id); return;
        }
        default:{
            res.redirect("/");
        }
    }

});

app.post('/roomcreator/upload', upload.single('file'), async (req, res) => {
    try {
        const file = req.file;
        console.log(req.file);
        if (!file) {
            return res.status(400).send('No file uploaded.');
        }
        let id = req.query.room;
        let room = rooms[id];

        if(room){
            const filePath = path.resolve(file.path);
            const fileContent = await fs.readFile(filePath, 'utf-8');
            const jsonData = JSON.parse(fileContent);

            const jsonString = JSON.stringify(jsonData);

            let pom = jsonString;

            room.game.sendJson(pom);
            room.game.send("load_json");
            let aaa = receive(room.game);
            console.log(aaa);

            if (isstate(room.game, ["playing"])) {
                res.redirect("/game2?room="+id);
                return;
            }

        }
        res.redirect(`/roomcreator/?room=${req.query.room}`);
    } catch (err) {
        console.error(err);
        res.status(500).send('An error occurred during file upload.');
    }
});

app.get('/roomcreator/pieces', async (req, res) => {
    let {username, userid} = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (room == undefined || !isstate(room.game, ["initpieces"])){
        res.redirect("/");
        return;
    }

    let pieces = await get_pieces();

    room.game.send("list")
    let pieces_in_game = receive2(room.game)
    res.render('roomcreator_pieces', { pieces, username, userid, pieces_in_game,id });
});

app.post('/roomcreator/pieces', async (req, res) => {
    let { username, userid } = get_user(req, res);
    let { choice } = req.body; // Wybrana gra lub "empty"

    var id = req.query.room;
    var room = rooms[id];

    if (room == undefined){
        res.redirect("/");
        return;
    }

    if (choice == "empty") {
        room.game.send("continue");
        receive(room.game);
        res.redirect(`/roomcreator/setup?room=${id}`);
    }
    else {
        if (room && userid === room.ID_first) {
            let piece_json = (await get_piece(choice))[0]["json"];
            console.log("piece_json: ", piece_json);
            room.game.sendJson(piece_json);
            room.game.send(`load_json`);
            receive(room.game);
        }

        res.redirect(`/roomcreator/pieces?room=${id}`);
    }
});

app.post('/roomcreator/pieces/upload', upload.single('file'), async (req, res) => {
    try {
        const file = req.file;

        if (!file) {
            return res.status(400).send('No file uploaded.');
        }
        let id = req.query.room;
        let room = rooms[id];

        if(room){
            const filePath = path.resolve(file.path);
            const fileContent = await fs.readFile(filePath, 'utf-8');
            const jsonData = JSON.parse(fileContent);

            const jsonString = JSON.stringify(jsonData);
            let pom = "verifypiece "+jsonString;
            room.game.send(pom);
            receive(room.game);
        }
        res.redirect(`/roomcreator/pieces?room=${req.query.room}`);
    } catch (err) {
        console.error(err);
        res.status(500).send('An error occurred during file upload.');
    }
});

app.post('/roomcreator/pieces/create', (req, res) => {
    let { username, userid } = get_user(req, res);
    let { choice } = req.body; // Wybrana gra lub "empty"

    var id = req.query.room;
    var room = rooms[id];

    if (room == undefined){
        res.redirect("/");
        return;
    }

    room.game.send("create");
    receive(room.game);
    res.redirect(`/piececreator?room=${id}`);
});

app.get('/piececreator', (req, res) => {
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["piececreator"])){
        res.redirect(`/`);
        return;
    }

    room.game.send("show id"); let pid = receive3(room.game);
    room.game.send("show name"); let name = receive3(room.game);
    room.game.send("show deathrattle"); let dr = receive3(room.game);
    room.game.send("show battlecry"); let bc = receive3(room.game);
    room.game.send("show passive"); let pas = receive3(room.game);
    room.game.send("show onmove"); let om = receive3(room.game);
    room.game.send("show aftermove"); let am = receive3(room.game);
    room.game.send("show onkill"); let ok = receive3(room.game);
    room.game.send("show possiblemoves"); let pm = receive3(room.game);
    room.game.send("show movecondition"); let mc = receive3(room.game);
    room.game.send("show memory"); let mem = receive3(room.game);

    res.render('piececreator', {pid,name,dr,bc,pas,om,am,ok,pm,mc,mem, id});
})

app.post('/piececreator', (req, res) =>{
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["piececreator"])){
        res.redirect(`/`);
        return;
    }

    let field= req.body["field"];
    console.log("field: ", field);

    room.game.send(`add `+field);
    let x = receive(room.game);
    room.game.send("getstatus");
    let stat = room.game.receive();

    if (stat == "movecreator"){
        res.redirect("/movecreator/?room="+id)
    }
    else if(stat == "rulecreator"){
       res.redirect('/rulecreator/?room='+id);
    }
    else{
        res.redirect('/piececreator/?room='+id);
    }
});

app.post('/piececreator/set', (req, res) => {
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["piececreator"])){
        res.redirect(`/`);
        return;
    }

    let pid = req.body["pid"]; let pname = req.body["pname"];
    room.game.send("set id "+pid); let r1 = receive(room.game);
    room.game.send("set name "+pname); let r2 = receive(room.game);

    console.log(r1, r2);
    res.redirect('/piececreator/?room='+id);
});

app.post('/piececreator/button', (req, res) => {
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["piececreator"])){
        res.redirect(`/`);
        return;
    }

    let command = req.body["choice"];
    room.game.send(command);
    let x = receive(room.game);
    console.log(x);
    room.game.send("getstatus");
    let stat = room.game.receive();
    console.log(stat);
    switch (stat){
        case "rulecreator":{
            res.redirect("/rulecreator/?room="+id); return;
        }
        case "initpieces":{
            res.redirect("/roomcreator/pieces/?room="+id); return;
        }
        case "piececreator":{
            res.redirect("/piececreator/?room="+id); return;
        }
        default:{
            res.redirect("/");
        }
    }

});

app.get('/movecreator', (req, res) => {
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["movecreator"])){
        res.redirect(`/`);
        return;
    }

    room.game.send("show condition"); let cond = receive3(room.game);
    room.game.send("show consequences"); let cons = receive3(room.game);

    res.render('movecreator', {cond, cons})
});

app.post('/movecreator', (req, res) => {
    let { username, userid } = get_user(req, res);
    var id = req.query.room;
    var room = rooms[id];

    if (!room || room.ID_first!=userid || !isstate(room.game, ["movecreator"])){
        res.redirect(`/`);
        return;
    }

    let command = req.body["choice"];
    room.game.send(command);
    let x = receive(room.game);
    console.log(x);
    room.game.send("getstatus");
    let stat = room.game.receive();
    console.log(stat);
    switch (stat){
        case "rulecreator":{
            res.redirect("/rulecreator/?room="+id); return;
        }
        case "initpieces":{
            res.redirect("/roomcreator/pieces/?room="+id); return;
        }
        case "piececreator":{
            res.redirect("/piececreator/?room="+id); return;
        }
        case "movecreator":{
            res.redirect("/movecreator/?room="+id); return;
        }
        default:{
            res.redirect("/");
        }
    }

});

app.get('/roomcreator/setup', (req, res) => {
    let {username, userid} = get_user(req, res);
    var id = req.query.room;
    if (!id) {
        res.redirect('/');
        return;
    }
    var room = rooms[id];

    if (!room || userid !== room.ID_first || !isstate(room.game, "placepieces")) {
        res.redirect('/');
        return;
    }

    room.game.send("show");
    let board = receive(room.game);
    room.game.send("list");
    let pieces_in_game = receive2(room.game);
    room.game.send("getdimensions");
    let dimensions = receive2(room.game);
    let dims = dimensions.split(",");
    res.render('roomcreator_setup', { room, username, userid, board, pieces_in_game, dims });
});

app.post('/roomcreator/setup', (req, res) => {
    let { username, userid } = get_user(req, res);
    let figure = req.body["figureName"]; let x = req.body["Xcord"]; let y = req.body["Ycord"]; let col = req.body["color"];
    var id = req.query.room;
    var room = rooms[id];

    if (room && userid === room.ID_first) {

        let tmp = `${figure} ${x},${y} ${col}`;
        console.log("setup:", tmp);
        room.game.send(`place ${tmp}`);
        console.log(receive(room.game));
    }

    res.redirect(`/roomcreator/setup?room=${id}`);
});

app.post('/roomcreator/setup/continue', async (req, res) => {
    try {
        let id = req.query.room;
        let room = rooms[id];

        room.game.send("continue");
        receive(room.game);
        res.redirect("/game2?room="+id);
    } catch (err) {
        console.error(err);
        res.status(500).send('An error occurred during redirect');
    }
});

app.post('/roomcreator/setup/export', async (req, res) => {
    try {
        let id = req.query.room;
        let room = rooms[id];

        room.game.send("export");
        let ret = receive3(room.game);
        const name = `game_${id}.json`;
        res.setHeader('Content-Disposition', `attachment; filename=${name}`);
        res.setHeader('Content-Type', 'application/json');
        res.send(ret);
    } catch (err) {
        console.error(err);
        res.status(500).send('An error occurred during game export.');
    }
});

app.post('/roomcreator/setup/share', async (req, res) => {
    try {
        let id = req.query.room;
        let room = rooms[id];

        let nazwa = req.body["nazwij"];
        room.game.send("export");
        let ret = receive3(room.game);
        insert_game(nazwa, ret);
        res.redirect(`/roomcreator/setup?room=${id}`);
    } catch (err) {
        console.error(err);
        res.status(500).send('An error occurred during file upload.');
    }
});

app.get( '/', (req, res) => {
    var roomnumber = roomslist.length;
    let {username, userid} =get_user(req,res);
    res.render('index', {roomslist,newroom: undefined,imie:username,roomnumber});
});

app.post('/', (req, res) => {
    var newroom = req.body.newroom;
    var existingroom = req.body.existingroom;
    var imie = req.body.imie;
    var id;
    let {username,userid} = get_user(req,res);

    if(imie){ //TODO decide what to do with unlogged user
        res.cookie('username', imie);
    }

    if (newroom)
    {
        if(rooms[newroom.toString()]==undefined){

            id = v4().toString();
            console.log("creating new room id:",id);
            var r = new room(newroom.toString(),id);
            r.ID_first=userid;
            rooms[id] = r;
            rooms[newroom.toString()] = r;
            roomslist.push(r);

        }
    }
    else if (existingroom)
    {
        id = existingroom.toString();
        if (!rooms[id].ID_second && rooms[id].ID_first!=userid)
        {
            rooms[id].ID_second = userid;
        }
    }

    res.redirect("/rooms?room="+id);
});

io.on('connection', function(socket) {

    socket.on('join room', function(roomid) {
        socket.join(roomid.toString());
        for(var i=0; i<roomslist.length;i++){
            if(roomid==roomslist[i].room_ID) roomslist[i].players++;
        }
    });

    socket.on('chat-message', function(napis,napis2,roomid,userid){
        var room = rooms[roomid];
        if(room){
            if((room.ID_first && room.ID_first==userid ) || (room.ID_second && room.ID_second==userid)){
                io.to(roomid.toString()).emit('board', napis,napis2);
            }
        }
    });

    socket.on('disconnecting', () => {
        let pokoje = socket.rooms;

        for(var i=0; i<roomslist.length;i++){
            if(pokoje.has(roomslist[i].room_ID)){
                roomslist[i].players--;
                if(roomslist[i].players<=0){
                    rooms[roomslist[i].name] = undefined;
                    roomslist.splice(i,1); ;
                }
            }
        }

    });
});

server.listen(8000);
