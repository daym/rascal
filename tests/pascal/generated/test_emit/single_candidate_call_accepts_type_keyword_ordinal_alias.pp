unit u;
interface
type
  tsuperregister = type word;
const
  RS_EAX = $00;
procedure take(r : tsuperregister);
procedure run(w : word);
implementation
procedure take(r : tsuperregister); begin end;
procedure run(w : word);
begin
  take(RS_EAX);
  take(w);
end;
end.
