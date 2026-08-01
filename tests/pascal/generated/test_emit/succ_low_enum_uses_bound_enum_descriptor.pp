unit u;
interface
type tkind = (first = 2, second = 5);
function nextkind : tkind;
implementation
function nextkind : tkind;
begin
  nextkind := succ(low(tkind));
end;
end.
