unit nx;
interface
uses ncon, globals;
type
  tx = class(treal)
    procedure check;
  end;
implementation
procedure tx.check;
begin
  if is_number_float(value_real) then
    ;
end;
end.
