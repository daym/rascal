unit u;
interface
type
  tfoo = class
    class procedure basehook; virtual; abstract;
    class procedure classy; override;
  end;
implementation
end.
