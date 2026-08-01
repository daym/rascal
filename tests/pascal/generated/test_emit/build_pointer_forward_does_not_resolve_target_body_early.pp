unit u;
interface
type
  PFoo = ^TFoo;
  TBar = record end;
  TFoo = record
    Bar : TBar;
  end;
implementation
end.
