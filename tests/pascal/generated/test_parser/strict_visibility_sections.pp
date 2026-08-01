unit u;
interface
type
  tfoo = class
  strict protected
    procedure hook;
  strict private
    secret : integer;
  public
    value : integer;
  end;
implementation
end.
