unit u;
interface
{$interfaces corba}
{$interfaces default}
type
  irefcounted = interface
    procedure addref;
  end;
implementation
end.
