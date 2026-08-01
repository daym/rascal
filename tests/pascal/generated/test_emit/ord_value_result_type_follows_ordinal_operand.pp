unit u;
interface
type tcolor = (red, green);
procedure run(w : word; wc : widechar; bb : bytebool; wb : wordbool; c : tcolor);
implementation
function pick_word(x : word) : byte;
begin
  pick_word := 1;
end;
function pick_word(x : longint) : byte;
begin
  pick_word := 2;
end;
function pick_shortint(x : shortint) : byte;
begin
  pick_shortint := 1;
end;
function pick_shortint(x : byte) : byte;
begin
  pick_shortint := 2;
end;
function pick_smallint(x : smallint) : byte;
begin
  pick_smallint := 1;
end;
function pick_smallint(x : word) : byte;
begin
  pick_smallint := 2;
end;
function pick_longint(x : longint) : byte;
begin
  pick_longint := 1;
end;
function pick_longint(x : byte) : byte;
begin
  pick_longint := 2;
end;
procedure run(w : word; wc : widechar; bb : bytebool; wb : wordbool; c : tcolor);
var b : byte;
begin
  b := pick_word(ord(w));
  b := pick_word(ord(wc));
  b := pick_shortint(ord(bb));
  b := pick_smallint(ord(wb));
  b := pick_longint(ord(c));
end;
end.
