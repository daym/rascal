unit u;
interface
procedure take_cardinal(c : cardinal);
procedure take_double(d : double);
procedure run(p : pointer; e : extended);
implementation
procedure take_cardinal(c : cardinal); begin end;
procedure take_double(d : double); begin end;
procedure run(p : pointer; e : extended);
begin
  take_cardinal(ptruint(p));
  take_double(e);
  take_double(double(e));
end;
end.
