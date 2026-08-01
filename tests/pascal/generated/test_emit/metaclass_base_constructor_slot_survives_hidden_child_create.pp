unit u;
interface
type
  tbase = class
    constructor create;
  end;
  tchild = class(tbase)
    constructor create(n : integer);
  end;
  tbaseclass = class of tbase;
var
  cls : tbaseclass;
  inst : tbase;
implementation
constructor tbase.create;
begin
end;
constructor tchild.create(n : integer);
begin
end;
begin
  cls := tchild;
  inst := cls.create;
end.
