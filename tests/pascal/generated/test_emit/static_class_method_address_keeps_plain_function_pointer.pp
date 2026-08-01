unit u;
interface
type
  tproc = function : integer;
  tbase = class
    class function kind : integer; virtual;
  end;
procedure bind(var p : tproc);
implementation
class function tbase.kind : integer;
begin
  kind := 1;
end;
procedure bind(var p : tproc);
begin
  p := @tbase.kind;
end;
end.
