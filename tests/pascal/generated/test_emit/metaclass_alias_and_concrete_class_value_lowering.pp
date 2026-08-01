unit u;
interface
type
  tbase = class
    constructor create(n : integer);
    class function load(n : integer) : integer;
  end;
  tchild = class(tbase)
  end;
  tbaseclass = class of tbase;
var
  cls : tbaseclass;
  inst : tbase;
implementation
constructor tbase.create(n : integer);
begin
end;
class function tbase.load(n : integer) : integer;
begin
  load := n;
end;
begin
  cls := tchild;
  inst := cls.create(1);
  if assigned(cls) then
    inst := cls.create(2);
end.
