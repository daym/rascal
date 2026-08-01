unit u;
interface
type
  tbase = class
  end;
  tchild = class(tbase)
  end;
  tunrelated = class
  end;
  tbaseclass = class of tbase;
procedure takebase(c : tbaseclass);
procedure takeany(c : tclass);
implementation
procedure takebase(c : tbaseclass);
begin
end;
procedure takeany(c : tclass);
begin
end;
begin
  takebase(tchild);
  takebase(tunrelated);
  takeany(tunrelated);
end.
