unit u;
interface
type
  titem = object
    value : longint;
    function getvalue : longint;
  end;
  tslot = object
    secidx : longint;
  end;
  pitem = ^titem;
  pslot = ^tslot;
  prec = ^trec;
  trec = packed record
    item : pitem;
    kind : longint;
  end;
  tslotarray = array[0..3] of pslot;
function read(p : prec; var slots : tslotarray) : longint;
implementation
function titem.getvalue : longint;
begin
  getvalue := value;
end;
function read(p : prec; var slots : tslotarray) : longint;
begin
  if assigned(p^.item) then
    read := p^.item^.value + p^.item^.getvalue + slots[p^.kind]^.secidx
  else
    read := 0;
end;
end.
