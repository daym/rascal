unit u;
interface
type
  tcompare = function(a, b : pointer) : integer;
  tlist = class
    procedure sort(compare : tcompare);
  end;
function byaddress(a, b : pointer) : longint;
procedure run(list : tlist);
implementation
procedure tlist.sort(compare : tcompare);
begin
end;
function byaddress(a, b : pointer) : longint;
begin
  byaddress := 0;
end;
procedure run(list : tlist);
begin
  list.sort(@byaddress);
end;
end.
