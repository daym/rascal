unit u;
interface
type
  tproc = function : integer;
  tbase = class
    class function kind : integer; virtual;
  end;
  tbaseclass = class of tbase;
procedure bind(cls : tbaseclass; var p : tproc; var q : pointer);
implementation
class function tbase.kind : integer;
begin
  kind := 1;
end;
procedure bind(cls : tbaseclass; var p : tproc; var q : pointer);
begin
  p := @cls.kind;
  p := cls.kind;
  q := @cls.kind;
end;
end.
