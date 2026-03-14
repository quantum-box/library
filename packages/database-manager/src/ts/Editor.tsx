import useSWR from "swr";
import { useGrpc } from "./useGrpc";
import { Data, Property } from "./gen/v1/database_manager_pb";

import { Button, Popover, Cross2Icon } from "ui";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import z from "zod";

export function Editor(props: { tenantId: string }) {
  return (
    <div>
      <div className="flex justify-between">
        <h1 className="text-xl font-semibold">Database</h1>
        <Button>Create</Button>
      </div>
      <DatabaseList tenantId={props.tenantId} />
    </div>
  );
}

function DatabaseList(props: { tenantId: string }) {
  const { grpc, headers } = useGrpc(props.tenantId);
  const { data } = useSWR("database", () =>
    grpc.findDatabases({}, { headers })
  );
  if (!data) return <div>Loading...</div>;
  return (
    <ul>
      {data.databases.map((v) => (
        <li key={v.id}>{v.name}</li>
      ))}
    </ul>
  );
}

export function DatabaseView(props: { databaseId: string; tenantId: string }) {
  const { grpc, headers } = useGrpc(props.tenantId);
  const { data } = useSWR(["database", props.databaseId], () =>
    grpc.getDatabase({ databaseId: props.databaseId }, { headers })
  );
  const { data: definition } = useSWR(["propertyData", props.databaseId], () =>
    grpc.getDatabaseDefinition({ databaseId: props.databaseId }, { headers })
  );
  if (!data) return <div>Loading...</div>;
  return (
    <div>
      <h1>{data.database?.name}</h1>
      <table className="overflow-x-auto">
        <thead>
          <tr>
            {definition?.properties.map((v) => (
              <TableHead key={v.id} property={v} />
            ))}
          </tr>
        </thead>
        <tbody>
          {data.data.map((v) => (
            <TableRow
              key={v.id}
              data={v}
              properties={definition?.properties || []}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function TableHead(props: { property: Property }) {
  const {
    register,
    handleSubmit,
    getValues,
    formState: { errors },
  } = useForm<Property>({
    defaultValues: props.property,
    resolver: zodResolver(
      z.object({
        name: z.string().min(1).max(255),
      })
    ),
  });
  return (
    <th className="whitespace-nowrap hover:bg-gray-100 p-1 px-6 cursor-pointer w-40 border">
      <Popover.Root>
        <Popover.Trigger asChild>
          <button
            className="w-full text-gray-500 font-semibold"
            aria-label="Update dimensions"
          >
            {getValues("name")}
          </button>
        </Popover.Trigger>

        <Popover.Portal>
          <Popover.Content
            className="relative bg-white shadow-lg p-2 border-gray-300 mt-1 border rounded"
            sideOffset={5}
          >
            <div className="flex flex-col">
              <fieldset className="Fieldset">
                <input
                  className="bg-gray-200 border-gray-400 px-1"
                  {...register("name")}
                  id="aa"
                />
              </fieldset>
              <PopoverItem>編集</PopoverItem>
              <PopoverItem>フィルタ</PopoverItem>
              <PopoverItem>並べ替え</PopoverItem>
              <PopoverItem>非表示</PopoverItem>
            </div>
          </Popover.Content>
        </Popover.Portal>
      </Popover.Root>
    </th>
  );
}

function PopoverItem(props: { children: React.ReactNode }) {
  return (
    <button className="text-left hover:bg-gray-200 px-2 py-1 rounded">
      {props.children}
    </button>
  );
}

function TableRow(props: { data: Data; properties: Property[] }) {
  return (
    <tr key={props.data.id}>
      {props?.properties.map((p) => (
        <Td key={p.id}>
          {props.data.propertyData.find((pd) => pd.propertyId == p.id)?.value}
        </Td>
      ))}
    </tr>
  );
}

function Td(props: { wrap?: boolean; children: React.ReactNode }) {
  const wrapStyle = !props.wrap
    ? "whitespace-nowrap overflow-hidden overflow-ellipsis"
    : "";
  return (
    <td className={`p-1 ${wrapStyle} max-w-0 border`}>{props.children}</td>
  );
}

export function DataView(props: {
  databaseId: string;
  tenantId: string;
  dataId: string;
}) {
  const { grpc, headers } = useGrpc(props.tenantId);
  const { data } = useSWR(["data", props.dataId], () =>
    grpc.getData({ dataId: props.dataId }, { headers })
  );

  if (!data) return <div>Loading...</div>;
  return (
    <div>
      <h1>{data.name}</h1>

      <ul>
        {data.propertyData.map((v) => (
          <li key={v.propertyId}>{v.value}</li>
        ))}
      </ul>
    </div>
  );
}
