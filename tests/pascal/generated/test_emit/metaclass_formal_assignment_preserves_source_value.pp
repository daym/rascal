unit u;
interface
type
  tabstractlinker = class
  end;
  tabstractlinkerclass = class of tabstractlinker;
  tsysteminfo = record
    linkextern : tabstractlinkerclass;
  end;
procedure registerexternallinker(var system_info : tsysteminfo; c : tabstractlinkerclass);
implementation
procedure registerexternallinker(var system_info : tsysteminfo; c : tabstractlinkerclass);
begin
  system_info.linkextern := c;
end;
end.
