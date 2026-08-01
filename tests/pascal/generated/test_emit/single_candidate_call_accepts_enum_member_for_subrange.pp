unit u;
interface
type
  tloc = (loc_void, loc_register, loc_reference);
  tnoref = low(tloc)..pred(loc_reference);
  tlocation = record
    loc : tloc;
  end;
procedure location_reset(var location : tlocation; loc : tnoref);
implementation
procedure location_reset(var location : tlocation; loc : tnoref);
begin
  location.loc := loc;
end;
procedure run;
var location : tlocation;
begin
  location_reset(location, loc_void);
end;
end.
