unit u;
interface
type
  tfoo = class
  protected
    function define(out wasdefined: boolean): integer;
  public
    name: string;
    procedure g; virtual;
    message, external: integer;
  end;
implementation
end.
