unit u;
interface
type
  tderef = object
    dataidx : longint;
    procedure reset;
    function resolve : pointer;
  end;
  titem = record
    case byte of
      0 : (symderef : tderef);
      1 : (other : longint);
  end;
function run(var item : titem) : pointer;
implementation
procedure tderef.reset;
begin
  dataidx := 0;
end;
function tderef.resolve : pointer;
begin
  resolve := nil;
end;
function run(var item : titem) : pointer;
begin
  with item do begin
    symderef.reset;
    run := symderef.resolve;
  end;
end;
end.
