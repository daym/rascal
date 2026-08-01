unit u;
interface
type
  touter = class
  public
    type
      tinner = class
        procedure doit;
      end;
    procedure use(v : tinner);
  end;
implementation
procedure touter.tinner.doit;
begin
end;
procedure touter.use(v : tinner);
begin
  v.doit;
end;
end.
