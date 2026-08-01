unit u;
interface
type
  thandler = class
    type
      ttemps = record value : integer; end;
    class procedure get_exception_temps(var t : ttemps); virtual;
  end;
  thandlerclass = class of thandler;
var
  c : thandlerclass;
procedure use;
implementation
class procedure thandler.get_exception_temps(var t : ttemps);
begin
  t.value := 1;
end;
procedure use;
var tmp : thandler.ttemps;
begin
  c.get_exception_temps(tmp);
end;
end.
