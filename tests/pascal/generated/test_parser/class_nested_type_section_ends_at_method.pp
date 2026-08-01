unit u;
interface
type
  touter = class
  public
    type
      tinner = record
        value : integer;
      end;
    procedure use(v : tinner);
  end;
implementation
procedure touter.use(v : tinner);
begin
end;
end.
