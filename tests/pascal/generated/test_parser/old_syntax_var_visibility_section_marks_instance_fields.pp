unit u;
interface
type
  touter = class
    type
      tinner = record
        value : integer;
      end;
    var protected
      fvalue : tinner;
  end;
implementation
end.
