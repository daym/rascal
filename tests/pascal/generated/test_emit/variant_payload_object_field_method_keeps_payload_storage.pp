unit u;
interface
type
  tderef = object
    dataidx : longint;
    procedure reset;
    function resolve : pointer;
  end;
  pitem = ^titem;
  titem = record
    case byte of
      0 : (symderef : tderef);
      1 : (value : longint);
  end;
function run(hp : pitem) : pointer;
implementation
procedure tderef.reset;
begin
  dataidx := 0;
end;
function tderef.resolve : pointer;
begin
  resolve := nil;
end;
function run(hp : pitem) : pointer;
begin
  hp^.symderef.reset;
  run := hp^.symderef.resolve;
end;
end.
