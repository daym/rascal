unit u;
interface
type
  tbase = class
    class function kind : integer; virtual;
  end;
  tchild = class(tbase)
    class function kind : integer; override;
  end;
  tbaseclass = class of tbase;
procedure demo(cls : tbaseclass; var i : integer);
implementation
class function tbase.kind : integer;
begin
  kind := 1;
end;
class function tchild.kind : integer;
begin
  kind := inherited kind + 1;
end;
procedure demo(cls : tbaseclass; var i : integer);
begin
  i := cls.kind;
end;
end.
