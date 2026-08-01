unit u;
interface
{$interfaces corba}
type
  idecorator = interface
    function lineprefix : ansistring;
  end;
  twriter = class
    decorator : idecorator;
    procedure writeansistring(const s : ansistring);
    procedure run;
  end;
implementation
procedure twriter.writeansistring(const s : ansistring);
begin
end;
procedure twriter.run;
begin
  writeansistring(decorator.lineprefix);
end;
end.
