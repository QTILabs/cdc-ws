import { ParentProps } from "solid-js";

export default function RootLayout(props: ParentProps) {
  return <>{props.children}</>;
}