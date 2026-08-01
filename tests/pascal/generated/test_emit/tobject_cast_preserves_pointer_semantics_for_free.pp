unit u;
interface
type
  tbox = class
    p : pointer;
    destructor destroy; override;
  end;
implementation
destructor tbox.destroy;
begin
  if assigned(p) then
    tobject(p).free;
  inherited destroy;
end;
end.
