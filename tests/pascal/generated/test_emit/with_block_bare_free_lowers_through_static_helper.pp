unit u;
interface
type
  TFoo = class
  end;
procedure run(obj : TFoo);
implementation
procedure run(obj : TFoo);
begin
  with obj do begin
    Free;
  end;
end;
end.
