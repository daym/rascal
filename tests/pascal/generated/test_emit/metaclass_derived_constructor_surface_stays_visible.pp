unit u;
interface
type
  tbase = class
  end;
  tchild = class(tbase)
    constructor create(n : integer);
  end;
  tchildclass = class of tchild;
var
  cls : tchildclass;
  inst : tchild;
implementation
constructor tchild.create(n : integer);
begin
end;
begin
  inst := cls.create(7);
end.
