unit u;
interface
type
  tbase = class
  public
    y : integer;
    class var x : integer;
    function getx : integer;
  end;
  tne = class(tbase)
  public
    class var z : integer;
  end;
procedure run(b : tbase; n : tne);
implementation
function tbase.getx : integer;
begin
  getx := x;
end;
procedure run(b : tbase; n : tne);
begin
  tbase.x := 1;
  tne.x := 2;
  b.x := 3;
  b.y := 4;
  n.z := 5;
end;
end.
