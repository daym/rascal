unit u;
interface
type
  tbase = class
  end;
  tbaseclass = class of tbase;
procedure put(p : pointer);
procedure demo(cls : tbaseclass);
implementation
procedure put(p : pointer);
begin
end;
procedure demo(cls : tbaseclass);
begin
  put(cls);
end;
end.
