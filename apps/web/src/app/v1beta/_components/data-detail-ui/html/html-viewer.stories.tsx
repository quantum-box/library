import type { Meta, StoryObj } from '@storybook/react'
import { RichTextViewer } from './viewer'

const meta = {
	title: 'Library/DataEditor/Html/Viewer',
	component: RichTextViewer,
} satisfies Meta<typeof RichTextViewer>

export default meta

type Story = StoryObj<typeof meta>

const html =
	'<div class="bn-block-group" data-node-type="blockGroup"><div class="bn-block-outer" data-node-type="blockOuter" data-id="974b8878-129b-41eb-a62d-07d482e8b76f"><div class="bn-block" data-node-type="blockContainer" data-id="974b8878-129b-41eb-a62d-07d482e8b76f"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content">Hello World</p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="26681681-4f14-4856-8744-7f71f524e773"><div class="bn-block" data-node-type="blockContainer" data-id="26681681-4f14-4856-8744-7f71f524e773"><div class="bn-block-content" data-content-type="heading" data-level="1"><h1 class="bn-inline-content">title</h1></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="f3cf53d5-3c7c-4414-8274-88af6f464b7a"><div class="bn-block" data-node-type="blockContainer" data-id="f3cf53d5-3c7c-4414-8274-88af6f464b7a"><div class="bn-block-content" data-content-type="heading" data-level="2"><h2 class="bn-inline-content">h2 title</h2></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="0db80b9f-fff3-4b1c-8651-990254e5160a"><div class="bn-block" data-node-type="blockContainer" data-id="0db80b9f-fff3-4b1c-8651-990254e5160a"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content"></p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="af657af5-ea2f-49d5-96f9-86e49394def7"><div class="bn-block" data-node-type="blockContainer" data-id="af657af5-ea2f-49d5-96f9-86e49394def7"><div class="bn-block-content" data-content-type="heading" data-level="3"><h3 class="bn-inline-content">h3 title</h3></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="2be31aeb-9fbc-4ba7-a9fd-d9b3f0c6906f"><div class="bn-block" data-node-type="blockContainer" data-id="2be31aeb-9fbc-4ba7-a9fd-d9b3f0c6906f"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content"></p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="68152e99-3f6f-496d-8136-193d240db473"><div class="bn-block" data-node-type="blockContainer" data-id="68152e99-3f6f-496d-8136-193d240db473"><div class="bn-block-content" data-content-type="bulletListItem"><p class="bn-inline-content">aaaa</p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="4fb935ed-be98-49fb-ab5d-c607cb1a8bfb"><div class="bn-block" data-node-type="blockContainer" data-id="4fb935ed-be98-49fb-ab5d-c607cb1a8bfb"><div class="bn-block-content" data-content-type="bulletListItem"><p class="bn-inline-content">aaa</p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="6955b2ea-9052-49cf-8195-a64a4e140457"><div class="bn-block" data-node-type="blockContainer" data-id="6955b2ea-9052-49cf-8195-a64a4e140457"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content"></p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="7cd9774b-e1df-45f1-b691-16e5e21bc7dd"><div class="bn-block" data-node-type="blockContainer" data-id="7cd9774b-e1df-45f1-b691-16e5e21bc7dd"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content"></p></div></div></div></div>'

export const Default: Story = {
	args: {
		html,
	},
}

export const NumberedListItem: Story = {
	args: {
		html: `
		<div class="bn-block-group" data-node-type="blockGroup">
    <div class="bn-block-outer" data-node-type="blockOuter" data-id="52b35c89-917d-45f3-b32a-31fd316b6cfa">
        <div class="bn-block" data-node-type="blockContainer" data-id="52b35c89-917d-45f3-b32a-31fd316b6cfa">
            <div class="bn-block-content" data-content-type="heading" data-level="1">
                <h1 class="bn-inline-content">data1</h1>
            </div>
        </div>
    </div>
    <div class="bn-block-outer" data-node-type="blockOuter" data-id="f5051c54-35ed-4f0f-82e5-e808c55526cf">
        <div class="bn-block" data-node-type="blockContainer" data-id="f5051c54-35ed-4f0f-82e5-e808c55526cf">
            <div class="bn-block-content" data-content-type="paragraph">
                <p class="bn-inline-content"></p>
            </div>
        </div>
    </div>
    <div class="bn-block-outer" data-node-type="blockOuter" data-id="66dfe253-6aa2-4685-9af3-1f9d1a737369">
        <div class="bn-block" data-node-type="blockContainer" data-id="66dfe253-6aa2-4685-9af3-1f9d1a737369">
            <div class="bn-block-content" data-content-type="numberedListItem" data-index="1">
                <p class="bn-inline-content">aaaa</p>
            </div>
        </div>
    </div>
    <div class="bn-block-outer" data-node-type="blockOuter" data-id="ad1c16e3-61cb-41c9-b0b1-9f781179a3ab">
        <div class="bn-block" data-node-type="blockContainer" data-id="ad1c16e3-61cb-41c9-b0b1-9f781179a3ab">
            <div class="bn-block-content" data-content-type="numberedListItem" data-index="null">
                <p class="bn-inline-content">aaaaaa</p>
            </div>
        </div>
    </div>
    <div class="bn-block-outer" data-node-type="blockOuter" data-id="8546fab5-71e9-4486-a44a-cfff4d5fd61d">
        <div class="bn-block" data-node-type="blockContainer" data-id="8546fab5-71e9-4486-a44a-cfff4d5fd61d">
            <div class="bn-block-content" data-content-type="numberedListItem" data-index="null">
                <p class="bn-inline-content">bbbbb</p>
            </div>
        </div>
    </div>
    <div class="bn-block-outer" data-node-type="blockOuter" data-id="79f5b706-6595-41a6-875c-48eaf4a0f7ba">
        <div class="bn-block" data-node-type="blockContainer" data-id="79f5b706-6595-41a6-875c-48eaf4a0f7ba">
            <div class="bn-block-content" data-content-type="numberedListItem" data-index="null">
                <p class="bn-inline-content">cccc</p>
            </div>
        </div>
    </div>
    <div class="bn-block-outer" data-node-type="blockOuter" data-id="0f9bb4e1-6830-4dcd-b906-4a7a77386f82">
        <div class="bn-block" data-node-type="blockContainer" data-id="0f9bb4e1-6830-4dcd-b906-4a7a77386f82">
            <div class="bn-block-content" data-content-type="paragraph">
                <p class="bn-inline-content"></p>
            </div>
        </div>
    </div>
</div>
		`,
	},
}
