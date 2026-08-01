unit u;
interface
type
  touter = class
    type tinner = class procedure run; end;
  end;
implementation
procedure touter.tinner.run;
begin
end;
end.
