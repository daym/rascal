unit u;
interface
type
  touter = class
  protected type
    tinner = record
      value : integer;
    end;
  protected var
    fvalue : tinner;
    property value : tinner read fvalue;
  end;
implementation
end.
