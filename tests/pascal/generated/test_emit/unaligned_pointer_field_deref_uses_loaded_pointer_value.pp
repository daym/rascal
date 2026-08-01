unit u;
interface
type
  titem = object
    value : longint;
    function getvalue : longint;
  end;
  pitem = ^titem;
  trec = packed record
    item : pitem;
  end;
function read(var r : trec) : longint;
implementation
function titem.getvalue : longint;
begin
  getvalue := value;
end;
function read(var r : trec) : longint;
begin
  if assigned(r.item) then
    read := r.item^.value + r.item^.getvalue
  else
    read := 0;
end;
end.
