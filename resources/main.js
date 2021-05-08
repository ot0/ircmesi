let update = 0;            
let noticeEnable = true;

function send_dice(){
    let xhr = new XMLHttpRequest();
    let url = "dice";        
    xhr.open("GET", url, true);
    xhr.onreadystatechange = function() {
        if(xhr.readyState == 4 && xhr.status != 200){
            alert("server busy!");
        }
    };
    xhr.send();        
}

function send_key(){
    let box = document.getElementById("sendMessage");
    if(event.keyCode != 13 || box.value == ""){
        return
    }
    send_message(box.value)
}

function send_message(value){
    let xhr = new XMLHttpRequest();
    let url = "send?q="+encodeURIComponent(value);        
    xhr.open("GET", url, true);
    xhr.onreadystatechange = function() {
        if(xhr.readyState == 4 && xhr.status == 200){
            document.getElementById("sendMessage").value = "";
        }
        else if(xhr.readyState == 4 && xhr.status != 200){
            alert("server busy!");
        }
    };
    xhr.send();
}

let urlreg = new RegExp("((https?|ftp)(:\/\/[-_.!~*\'()a-zA-Z0-9;\/?:\@&=+\$,%#]+))", "g");
function reptxt(text){
    return text
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(urlreg, "<a href='$1' target='_blank'>$1</a>");
}

function get_log(){
    let xhr = new XMLHttpRequest();
    let url = "/msg?id="+update      
    xhr.open("GET", url, true); 
    xhr.responseType = 'json';
    xhr.onreadystatechange = function() {
        if(xhr.readyState == 4){
            if(xhr.status == 200){
                let log = xhr.response;
                document.getElementById("topic").innerHTML = reptxt(log.topic);
                document.getElementById("member").innerHTML = reptxt(log.member);
                let msg = "";
                for(let line of log.log){
                    msg +="<div>" + reptxt(line) + "</div>";
                }

                document.getElementById("log_area").innerHTML = msg;
                update = log.id;
                if(noticeEnable && log.log[0].charAt(8)=="<"){
                    let n = new Notification(log.log[0]);
                    noticeEnable = false;
                    setTimeout(function(){
                        noticeEnable = true;
                    }, 1800*1000);
                }
            };
            setTimeout(get_log, 1000);
        }
    }
    xhr.send();
}

function searchlog(){
    let xhr = new XMLHttpRequest();
    let url = "/grep?q=" + encodeURIComponent(document.getElementById("search").value);
    xhr.open("GET", url, true); 
    xhr.responseType = 'json';
    xhr.onreadystatechange = function() {
        if(xhr.readyState == 4 && xhr.status == 200){
            let msg = "";
            let before = "";
            for(let gp of xhr.response){
                if(gp.type == "match"){
                    let d = gp.data;
                    if(before != d.path.text){
                        msg += "<div>" + "<a href=\"/log/" + d.path.text +"\" target=\"log\">" 
                            + d.path.text + "</a></div>"
                        before = d.path.text;
                    }
                    msg += "<div>" +d.line_number + ": " + reptxt(d.lines.text) + "</div>"
                }
            document.getElementById("search_area").innerHTML = msg;
            }
        }
    };
    xhr.send();
}

calendar = (function(){
    const weeks = ['日', '月', '火', '水', '木', '金', '土'];
    let date = new Date();
    let targetObj = null;
    let length = 84;
    
    let holidayList = {}
    function init(obj){
        targetObj = obj;

        let xhr = new XMLHttpRequest();
        xhr.open("GET", "https://holidays-jp.github.io/api/v1/date.json");
        xhr.responseType = "json";
        xhr.onload = function(){
            if(xhr.status == 200){
                console.log(xhr.response);
                holidayList = xhr.response;
            }
            show();
        }
        xhr.send(null);    
    }
    
    function date2str(dt, sep="-"){
        return [dt.getFullYear(),
            ("00" + (dt.getMonth()+1)).slice(-2),
            ("00" + dt.getDate()).slice(-2)
        ].join(sep);
    }

    function isHoliday(dt){
        //console.log(key);
        return holidayList[date2str(dt)];
    }

    function createCalendar() {
        let calendarHtml = '' // HTMLを組み立てる変数
        let start = date.getDay() + Math.ceil(length*2/3) 
        change(-start);
        let end = new Date(date.getTime() + length*24*60*60*1000);
        
        // 曜日の行を作成
        calendarHtml += '<table><thead><tr>'
        for (const week of weeks) {
            calendarHtml += '<th>' + week + '</th>';
        }
        calendarHtml += '</tr></thead><tbody>';

        let today = new Date();
        while (date < end) {
            calendarHtml += '<tr>'

            for (let d = 0; d < 7; d++) {
                let addClass = [];
                let name = "";
                let holidayname = isHoliday(date);
                //console.log(holidayname);
                if(holidayname){
                    addClass.push("holiday");
                    name += holidayname;
                }else if(d == 0 || d ==6){
                    addClass.push("holiday");
                }

                if(date.getFullYear() == today.getFullYear() &&
                    date.getMonth() == today.getMonth() &&
                    date.getDate() == today.getDate()
                ){
                    addClass.push("today");
                }

                if(name == ""){
                    name = "..."
                }
                let dn = date2str(date,"");
                calendarHtml += '<td class="'+ addClass.join(" ") +'"><div>' + 
                    (date.getMonth()+1) + '/' + date.getDate() +
                    ' <a href="https://irc.qpic.org/log/log' + dn +
                    '" target="log">log</a>' +
                    '</div><div class="holidayname">' + 
                    '<a href="log/irc' + dn + '.txt" target="log">' +
                    name + '</a></div>' +
                    '</td>'
                change(1);
            }
            calendarHtml += '</tr>'
        }
        calendarHtml += '</table>'
        change(start - length)
        return calendarHtml
    }

    function show() {
        targetObj.innerHTML = createCalendar();
        document.getElementById("calendar_dateinput").value = date2str(date);
    }

    function change(i){
        date.setDate(date.getDate() + i); 
    }

    function move(e) {
        if (e.target.id === 'calendar_prev') {
            change(-14)
        }

        if (e.target.id === 'calendar_next') {
            change(14)
        }
        show();
    }

    function setDate(e){
        date = new Date(e.target.value);
        //console.log(date);
        show();
    }

    function setToday(e){
        date = new Date();
        show();
    }

    return {
        init: init,
        show: show,
        move: move,
        setToday: setToday,
        setDate: setDate
    }
})();

document.addEventListener("DOMContentLoaded", function(event){
    get_log();

    document.getElementById('calendar_prev').addEventListener('click', calendar.move);
    document.getElementById('calendar_today').addEventListener('click', calendar.setToday);   
    document.getElementById('calendar_next').addEventListener('click', calendar.move);
    document.getElementById('calendar_dateinput').addEventListener("change", calendar.setDate);
    calendar.init(document.querySelector("#calendar"));

    if (Notification.permission === 'default') {
        Notification.requestPermission();
    }else if (Notification.permission === 'granted') {
        noticeEnable = true;
    }else{
        noticeEnable = false;
    }
});
