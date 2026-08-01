unit u;
interface
type
  treal = class
    type
      tinner = record value : longint; end;
    procedure ping(x : tinner);
  end;
  talias = treal;
implementation
procedure talias.ping(x : tinner);
begin
  x.value := 1;
end;
end.
