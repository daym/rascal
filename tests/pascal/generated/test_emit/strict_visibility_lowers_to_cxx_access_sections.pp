unit u;
interface
type
  tfoo = class
  protected
    plain : integer;
  strict protected
    procedure hook;
  strict private
    secret : integer;
  public
    value : integer;
  end;
implementation
procedure tfoo.hook;
begin
end;
end.
